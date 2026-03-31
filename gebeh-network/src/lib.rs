use std::collections::VecDeque;

use arraydeque::ArrayDeque;
use arrayvec::ArrayVec;
use gebeh_core::{Emulator, joypad::JoypadInput};
use gebeh_front_helper::{CloneMbc, EasyMbc};
use rkyv::rancor;

use crate::message::{
    ArchivedSerialMessage, DecompressedSerialMessage, MessageFromMaster, MessageFromSlave,
    SerialMessage,
};

pub mod message;

struct SynchroCycles {
    master: u64,
    slave: u64,
}

#[derive(Default)]
struct MessageFromMasterAcc {
    messages: Vec<(u8, Emulator, EasyMbc)>,
    session: bool,
}

type Snapshots = VecDeque<Snapshot>;

type Snapshot = (Emulator, EasyMbc);

// 6 seconds
const ROLLBACK_TRESHOLD: u64 = 4194304 * 6 / 4;
// 10 ms
const BATCH_PERIOD: u64 = 4194304 / 4 / 100;
const MAX_SNAPSHOT: usize = 240;
const ROLLBACK_SNAPSHOT_PERIOD: u64 = ROLLBACK_TRESHOLD / MAX_SNAPSHOT as u64;
const INPUTS_HISTORY_SIZE: usize = 50;

pub struct RollbackSerial {
    current_message: MessageFromMasterAcc,
    master_snapshots: Vec<(Emulator, EasyMbc)>,
    slave_snapshots: Snapshots,
    synchro_cycles: Option<SynchroCycles>,
    // it's not the actual input value at a given cycle, but WHEN the input changes
    // to avoid saving inputs every cycle
    inputs_history: Box<ArrayDeque<(u64, JoypadInput), INPUTS_HISTORY_SIZE>>,
    last_correction: u8,
}

impl Default for RollbackSerial {
    fn default() -> Self {
        Self {
            current_message: Default::default(),
            master_snapshots: Default::default(),
            slave_snapshots: Default::default(),
            synchro_cycles: Default::default(),
            inputs_history: Default::default(),
            last_correction: 0xff,
        }
    }
}

impl RollbackSerial {
    #[must_use]
    fn execute(&mut self, prediction: u8, cycles: u64) -> Option<MessageFromMaster> {
        let (_, first_snap, _) = self.current_message.messages.first()?;

        if cycles - first_snap.get_cycles() <= BATCH_PERIOD {
            return None;
        }

        self.master_snapshots
            .retain(|(snap, _)| cycles - snap.get_cycles() < ROLLBACK_TRESHOLD);
        let mut messages = core::mem::take(&mut self.current_message.messages).into_iter();
        let (first_byte, first_snap, first_mbc) = messages.next().unwrap();
        let first_cycle = first_snap.get_cycles();

        self.master_snapshots.push((first_snap, first_mbc));

        let mut messages_to_send = Vec::new();
        for (byte, emulator, mbc) in messages {
            messages_to_send.push((byte, emulator.get_cycles()));
            self.master_snapshots.push((emulator, mbc));
        }

        let msg_to_send = MessageFromMaster {
            first_message: (first_byte, first_cycle),
            messages: messages_to_send,
            prediction,
        };

        Some(msg_to_send)
    }

    fn add_snapshot(&mut self, snapshot: Snapshot) {
        if self.slave_snapshots.len() == MAX_SNAPSHOT {
            self.slave_snapshots.pop_front();
        }
        self.slave_snapshots.push_back(snapshot)
    }

    pub fn handle_msg_no_emulator(msg: &[u8]) -> Option<Box<[u8]>> {
        let msg = SerialMessage::deserialize(msg);
        if let ArchivedSerialMessage::FromMaster(msg) = msg.get()
            && msg.prediction != 0xff
        {
            Some(
                SerialMessage::FromSlave(MessageFromSlave {
                    correction: 0xff,
                    cycle: msg.first_message.1.to_native(),
                })
                .serialize(),
            )
        } else {
            None
        }
    }

    #[must_use]
    pub fn set_serial_messages(
        &mut self,
        msg: impl IntoIterator<Item = Box<[u8]>>,
        emulator: &mut Emulator,
        mbc: &mut EasyMbc,
    ) -> Vec<Box<[u8]>> {
        group_master_messages(msg.into_iter().map(|a| SerialMessage::deserialize(&a)))
            .flat_map(|msg| {
                match msg {
                    SerialMessage::FromMaster(msg) => {
                        let current_joypad = *emulator.get_joypad();
                        let messages = self.handle_message_from_master(emulator, mbc, msg);
                        emulator.set_joypad(current_joypad);
                        messages
                    }
                    SerialMessage::FromSlave(msg) => {
                        let (mut snap_emulator, snap_mbc) =
                            core::mem::take(&mut self.master_snapshots)
                                .into_iter()
                                .find(|(emulator, _)| emulator.get_cycles() == msg.cycle)
                                .expect("desync too big");
                        snap_emulator.set_joypad(*emulator.get_joypad());
                        *emulator = snap_emulator;
                        *mbc = snap_mbc;
                        log::info!(
                            "Correction from slave 0x{:02x} -> 0x{:02x}",
                            emulator.serial.slave_byte,
                            msg.correction
                        );
                        log::info!("Will emit serial {}", emulator.will_serial_emit_byte());

                        emulator.serial.slave_byte = msg.correction;
                        self.current_message.session = !self.current_message.session;
                        self.current_message.messages.clear();
                        emulator.execute(mbc.as_mut());

                        // TODO catchup, however the master will send a lot of messages without being able
                        // to receive the slave ones

                        Default::default()
                    }
                }
            })
            .collect()
    }

    #[must_use]
    fn handle_message_from_master(
        &mut self,
        emulator: &mut Emulator,
        mbc: &mut EasyMbc,
        msg: MessageFromMaster,
    ) -> Vec<Box<[u8]>> {
        if msg.prediction != self.last_correction {
            log::info!("Bad session");
            return Default::default();
        }

        let Some(synchro_cycles) = self.synchro_cycles.as_mut() else {
            log::info!("first batch");
            let slave_cycles = emulator.get_cycles();
            self.synchro_cycles = Some(SynchroCycles {
                master: msg.first_message.1,
                slave: slave_cycles,
            });

            let (msg_from_slave, correction) = self.advance_while_consuming_messages(
                msg,
                &mut [].as_slice(),
                emulator,
                mbc.as_mut(),
            );

            self.add_snapshot((emulator.clone(), mbc.clone_boxed()));

            if let Some(correction) = correction {
                self.last_correction = correction;
            }

            return msg_from_slave;
        };

        let current_cycle = emulator.get_cycles();

        let restore_cycle = synchro_cycles.slave + msg.first_message.1 - synchro_cycles.master;

        let snapshots = core::mem::take(&mut self.slave_snapshots);

        if let Some((snap_emulator, snap_mbc)) = snapshots
            .into_iter()
            .rev()
            .find(|(emulator, _)| emulator.get_cycles() <= restore_cycle)
        {
            *emulator = snap_emulator;
            *mbc = snap_mbc;
            self.add_snapshot((emulator.clone(), mbc.clone_boxed()));
        } else {
            panic!("big delay");
        };

        self.synchro_cycles = Some(SynchroCycles {
            master: self.synchro_cycles.as_ref().unwrap().master + emulator.get_cycles()
                - self.synchro_cycles.as_ref().unwrap().slave,
            slave: emulator.get_cycles(),
        });

        let inputs_history: Vec<_> = self
            .inputs_history
            .iter()
            .filter(|(cycle, _)| *cycle > emulator.get_cycles())
            .copied()
            .collect();
        let mut inputs_history = inputs_history.as_slice();

        let (mut messages, correction) =
            self.advance_while_consuming_messages(msg, &mut inputs_history, emulator, mbc.as_mut());

        self.add_snapshot((emulator.clone(), mbc.clone_boxed()));

        if correction.is_none() && current_cycle > emulator.get_cycles() {
            // catching up
            for _ in 0..(current_cycle - emulator.get_cycles()) {
                messages.extend(self.execute_and_take_snapshot(emulator, mbc.as_mut()));
                if let Some((cycle, input)) = inputs_history.first()
                    && *cycle == emulator.get_cycles()
                {
                    emulator.set_joypad(*input);
                    inputs_history = &inputs_history[1..];
                }
            }
        }

        if let Some(correction) = correction {
            self.last_correction = correction;
        }

        messages
    }

    #[must_use]
    fn advance_while_consuming_messages(
        &mut self,
        msg: MessageFromMaster,
        inputs_history: &mut &[(u64, JoypadInput)],
        emulator: &mut Emulator,
        mbc: &mut dyn CloneMbc<'static>,
    ) -> (Vec<Box<[u8]>>, Option<u8>) {
        let mut messages = Vec::new();
        for (byte, master_cycle) in std::iter::once(&msg.first_message)
            .chain(msg.messages.iter())
            .map(|a| (a.0, a.1))
        {
            for _ in 0..master_cycle - self.synchro_cycles.as_ref().unwrap().master {
                messages.extend(self.execute_and_take_snapshot(emulator, mbc));

                if let Some((cycle, input)) = inputs_history.first()
                    && *cycle == emulator.get_cycles()
                {
                    emulator.set_joypad(*input);
                    *inputs_history = &inputs_history[1..];
                }
            }

            self.synchro_cycles = Some(SynchroCycles {
                master: master_cycle,
                slave: emulator.get_cycles(),
            });

            let response = emulator
                .serial
                .set_msg_from_master(byte, &mut emulator.state);
            if response != msg.prediction {
                messages.push(
                    SerialMessage::FromSlave(MessageFromSlave {
                        correction: response,
                        cycle: master_cycle,
                    })
                    .serialize(),
                );
                return (messages, Some(response));
            }
        }
        (messages, None)
    }

    #[must_use]
    pub fn execute_and_take_snapshot(
        &mut self,
        emulator: &mut Emulator,
        mbc: &mut dyn CloneMbc<'static>,
    ) -> Option<Box<[u8]>> {
        let msg = self.execute(emulator.serial.slave_byte, emulator.get_cycles());

        if emulator.will_serial_emit_byte() {
            let emulator_clone = emulator.clone();
            let mbc_clone = mbc.clone_boxed();
            let byte = emulator.execute(mbc).unwrap();
            self.current_message
                .messages
                .push((byte, emulator_clone, mbc_clone));
        } else {
            emulator.execute(mbc);
        }

        let cycles = emulator.get_cycles();
        if cycles.is_multiple_of(ROLLBACK_SNAPSHOT_PERIOD) {
            self.add_snapshot((emulator.clone(), mbc.clone_boxed()));
        }

        msg.map(|msg| SerialMessage::FromMaster(msg).serialize())
    }

    pub fn save_input(&mut self, cycles: u64, joypad: JoypadInput) {
        let Err(arraydeque::CapacityError { element }) =
            self.inputs_history.push_back((cycles, joypad))
        else {
            return;
        };

        self.inputs_history.pop_front();
        self.inputs_history.push_back(element).unwrap();
    }
}

fn group_master_messages(
    messages: impl IntoIterator<Item = DecompressedSerialMessage>,
) -> impl Iterator<Item = SerialMessage> {
    messages
        .into_iter()
        .map(Some)
        .chain(core::iter::once(None))
        .scan::<Option<MessageFromMaster>, ArrayVec<SerialMessage, 2>, _>(
            None,
            |acc_serial_msg, serial_msg| {
                let Some(serial_msg) = serial_msg else {
                    return Some(
                        acc_serial_msg
                            .take()
                            .map(SerialMessage::FromMaster)
                            .into_iter()
                            .collect(),
                    );
                };
                let serial_msg = serial_msg.get();
                match (acc_serial_msg, serial_msg) {
                    (acc @ None, ArchivedSerialMessage::FromMaster(msg)) => {
                        *acc = Some(
                            rkyv::deserialize::<MessageFromMaster, rancor::Error>(msg).unwrap(),
                        );
                        Some(Default::default())
                    }
                    (None, ArchivedSerialMessage::FromSlave(msg)) => Some(
                        core::iter::once(SerialMessage::FromSlave(
                            rkyv::deserialize::<MessageFromSlave, rancor::Error>(msg).unwrap(),
                        ))
                        .collect(),
                    ),
                    (Some(acc), ArchivedSerialMessage::FromMaster(msg)) => {
                        if acc.prediction == msg.prediction {
                            acc.messages.extend(
                                core::iter::once(&msg.first_message)
                                    .chain(msg.messages.iter())
                                    .map(|a| (a.0, a.1.to_native())),
                            );
                            Some(Default::default())
                        } else {
                            Some(
                                core::iter::once(SerialMessage::FromMaster(core::mem::replace(
                                    acc,
                                    rkyv::deserialize::<MessageFromMaster, rancor::Error>(msg)
                                        .unwrap(),
                                )))
                                .collect(),
                            )
                        }
                    }
                    (acc @ Some(_), ArchivedSerialMessage::FromSlave(msg)) => Some(
                        [
                            SerialMessage::FromMaster(acc.take().unwrap()),
                            SerialMessage::FromSlave(
                                rkyv::deserialize::<MessageFromSlave, rancor::Error>(msg).unwrap(),
                            ),
                        ]
                        .into(),
                    ),
                }
            },
        )
        .flatten()
}
