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

struct MiamMessage {
    is_master: bool,
    cycle: u64,
    value: u8,
}

pub struct RollbackSerial {
    current_message: MessageFromMasterAcc,
    master_snapshots: Vec<(Emulator, EasyMbc)>,
    slave_snapshots: Snapshots,
    synchro_cycles: Option<SynchroCycles>,
    // it's not the actual input value at a given cycle, but WHEN the input changes
    // to avoid saving inputs every cycle
    inputs_history: Box<ArrayDeque<(u64, JoypadInput), INPUTS_HISTORY_SIZE>>,
    last_correction: u8,
    // Les from master sont fusionnés correctement
    // ceux qui n'ont pas la bonne correction sont ignorées
    // gérer le cas du switch slave <-> master qui peut foutre la merde car ça ignore
    // le concept de session
    messages_to_handle: VecDeque<MiamMessage>,
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
            messages_to_handle: Default::default(),
        }
    }
}

impl RollbackSerial {
    pub fn add_message(&mut self, msg: &[u8]) {
        let msg = SerialMessage::deserialize(msg);
        match msg.get() {
            ArchivedSerialMessage::FromMaster(msg) => {
                if msg.prediction != self.last_correction {
                    return;
                }
                core::iter::once((msg.first_message.0, msg.first_message.1.to_native()))
                    .chain(msg.messages.iter().map(|a| (a.0, a.1.to_native())))
                    .for_each(|(byte, cycle)| {
                        self.messages_to_handle.push_back(MiamMessage {
                            is_master: true,
                            cycle,
                            value: byte,
                        })
                    });
            }
            ArchivedSerialMessage::FromSlave(msg) => {
                self.messages_to_handle.push_back(MiamMessage {
                    is_master: false,
                    cycle: msg.cycle.to_native(),
                    value: msg.correction,
                });
            }
        }
    }

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
    pub fn fix_deviation_before_running(
        &mut self,
        emulator: &mut Emulator,
        mbc: &mut EasyMbc,
    ) -> Vec<Box<[u8]>> {
        let Some(msg) = self.messages_to_handle.front() else {
            return Default::default();
        };

        let current_cycle = emulator.get_cycles();

        if !msg.is_master {
            let (mut snap_emulator, snap_mbc) = core::mem::take(&mut self.master_snapshots)
                .into_iter()
                .find(|(emulator, _)| emulator.get_cycles() == msg.cycle)
                .expect("desync too big");
            snap_emulator.set_joypad(*emulator.get_joypad());
            *emulator = snap_emulator;
            *mbc = snap_mbc;
            log::info!(
                "Correction from slave 0x{:02x} -> 0x{:02x}",
                emulator.serial.slave_byte,
                msg.value
            );
            log::info!("Will emit serial {}", emulator.will_serial_emit_byte());

            emulator.serial.slave_byte = msg.value;
            self.current_message.session = !self.current_message.session;
            self.current_message.messages.clear();
            emulator.execute(mbc.as_mut());

            // catch up
            return (emulator.get_cycles()..current_cycle)
                .flat_map(|_| self.execute_and_take_snapshot(emulator, mbc.as_mut()))
                .collect();
        }

        let Some(synchro_cycles) = self.synchro_cycles.as_mut() else {
            log::info!("first batch");
            let slave_cycles = emulator.get_cycles();
            // will consume the messages later during the normal execution
            self.synchro_cycles = Some(SynchroCycles {
                master: msg.cycle.wrapping_add(1),
                slave: slave_cycles.wrapping_add(1),
            });

            return Default::default();
        };

        let restore_cycle = synchro_cycles.slave + msg.cycle - synchro_cycles.master;

        if restore_cycle >= current_cycle {
            return Default::default();
        }

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

        let mut messages = core::mem::take(&mut self.messages_to_handle);

        let (mut messages, correction) = self.advance_while_consuming_messages2(
            // ignore messages from slave we shouldn't receive them (will be replayed anyway)
            take_while_pop_front(&mut messages, |msg| {
                msg.is_master && msg.cycle <= current_cycle
            })
            .filter_map(|msg| msg.is_master.then_some((msg.cycle, msg.value))),
            &mut inputs_history,
            emulator,
            mbc.as_mut(),
        );

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
    fn advance_while_consuming_messages2(
        &mut self,
        messages_from_master: impl IntoIterator<Item = (u64, u8)>,
        inputs_history: &mut &[(u64, JoypadInput)],
        emulator: &mut Emulator,
        mbc: &mut dyn CloneMbc<'static>,
    ) -> (Vec<Box<[u8]>>, Option<u8>) {
        let mut messages = Vec::new();
        for (master_cycle, byte) in messages_from_master.into_iter() {
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
            if response != self.last_correction {
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
    ) -> Vec<Box<[u8]>> {
        let mut messages: Vec<Box<[u8]>> = self
            .execute(emulator.serial.slave_byte, emulator.get_cycles())
            .into_iter()
            .map(|msg| SerialMessage::FromMaster(msg).serialize())
            .collect();

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

        if let Some(msg) = self.messages_to_handle.front() {
            if !msg.is_master {
                panic!("pas possible de recevoir un slave");
            }
            if msg.cycle < cycles {
                panic!("les cycles maintenant ou plus tard")
            }
            if msg.cycle == cycles {
                let response = emulator
                    .serial
                    .set_msg_from_master(msg.value, &mut emulator.state);
                if response != self.last_correction {
                    messages.push(
                        SerialMessage::FromSlave(MessageFromSlave {
                            correction: response,
                            cycle: msg.cycle,
                        })
                        .serialize(),
                    );
                    self.messages_to_handle.clear();
                }
                self.messages_to_handle.pop_front();
            }
        }

        if cycles.is_multiple_of(ROLLBACK_SNAPSHOT_PERIOD) {
            self.add_snapshot((emulator.clone(), mbc.clone_boxed()));
        }

        messages
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

fn take_while_pop_front<'a, T, F>(
    deque: &'a mut VecDeque<T>,
    mut pred: F,
) -> impl Iterator<Item = T> + 'a
where
    F: FnMut(&T) -> bool + 'a,
{
    std::iter::from_fn(move || match deque.front() {
        Some(x) if pred(x) => deque.pop_front(),
        _ => None,
    })
}
