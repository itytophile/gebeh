use std::collections::VecDeque;

use gebeh_core::Emulator;
use gebeh_front_helper::{CloneMbc, EasyMbc};

use crate::{
    message::{ArchivedSerialMessage, MessageFromMaster, MessageFromSlave, SerialMessage},
    synchro_cycles::{CycleToSync, SynchroCycles},
};

pub mod message;
mod synchro_cycles;

#[derive(Default)]
struct MessageFromMasterAcc {
    messages: Vec<(u8, Emulator, EasyMbc)>,
    session: bool,
}

type Snapshots = VecDeque<Snapshot>;

type Snapshot = (Emulator, EasyMbc);

// 3 seconds
const ROLLBACK_TRESHOLD: u64 = 4194304 * 3 / 4;
// 10 ms
const BATCH_PERIOD: u64 = 4194304 / 4 / 100;
const MAX_SNAPSHOT: usize = (ROLLBACK_TRESHOLD / ROLLBACK_SNAPSHOT_PERIOD) as usize;
// half of a quantum batch duration
const ROLLBACK_SNAPSHOT_PERIOD: u64 = 4194304 / 4 * 128 / 48000 / 2;

enum MiamMessage {
    FromMaster(CycleToSync, u8),
    FromSlave(u64, u8),
}

pub struct RollbackSerial {
    current_message: MessageFromMasterAcc,
    master_snapshots: Vec<(Emulator, EasyMbc)>,
    slave_snapshots: Snapshots,
    synchro_cycles: Option<SynchroCycles>,
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
                        self.messages_to_handle
                            .push_back(MiamMessage::FromMaster(CycleToSync::new(cycle), byte))
                    });
            }
            ArchivedSerialMessage::FromSlave(msg) => {
                self.messages_to_handle.push_back(MiamMessage::FromSlave(
                    msg.cycle.to_native(),
                    msg.correction,
                ));
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

    // never try to "catch up" when there is a rollback, that's too hard for phone CPUs

    pub fn rollback_if_necessary(&mut self, emulator: &mut Emulator, mbc: &mut EasyMbc) {
        let Some(msg) = self.messages_to_handle.front() else {
            return Default::default();
        };

        let current_cycle = emulator.get_cycles();

        let (cycle, _) = match msg {
            MiamMessage::FromMaster(cycle, value) => (*cycle, *value),
            MiamMessage::FromSlave(cycle, value) => {
                let (mut snap_emulator, snap_mbc) = self
                    .master_snapshots
                    .drain(..)
                    .find(|(emulator, _)| emulator.get_cycles() == *cycle)
                    .expect("desync too big");
                snap_emulator.set_joypad(*emulator.get_joypad());
                *emulator = snap_emulator;
                *mbc = snap_mbc;

                emulator.serial.slave_byte = *value;
                self.current_message.session = !self.current_message.session;
                self.current_message.messages.clear();
                // to avoid the master to reemit a message already handled by the slave
                emulator.execute(mbc.as_mut());

                self.messages_to_handle.clear();

                return;
            }
        };

        let Some(synchro_cycles) = self.synchro_cycles.as_mut() else {
            return;
        };

        let restore_cycle = synchro_cycles.get_slave_cycle_from_master_cycle(cycle);

        if restore_cycle >= current_cycle {
            return;
        }

        let previous_input = *emulator.get_joypad();

        if let Some((snap_emulator, snap_mbc)) = self
            .slave_snapshots
            .drain(..)
            .rev()
            .find(|(emulator, _)| emulator.get_cycles() <= restore_cycle)
        {
            *emulator = snap_emulator;
            *mbc = snap_mbc;
        } else {
            panic!("big delay");
        };

        emulator.set_joypad(previous_input);
    }

    #[must_use]
    pub fn execute_and_take_snapshot(
        &mut self,
        emulator: &mut Emulator,
        mbc: &mut dyn CloneMbc<'static>,
    ) -> Vec<Box<[u8]>> {
        if emulator
            .get_cycles()
            .is_multiple_of(ROLLBACK_SNAPSHOT_PERIOD)
        {
            self.add_snapshot((emulator.clone(), mbc.clone_boxed()));
        }

        let mut messages = Vec::<Box<[u8]>>::new();

        if let Some(msg) = self.messages_to_handle.front() {
            match msg {
                MiamMessage::FromMaster(cycle_to_sync, value) => {
                    let synchro_cycle = self
                        .synchro_cycles
                        .get_or_insert(SynchroCycles::new(*cycle_to_sync, emulator.get_cycles()));
                    let synced_cycle =
                        synchro_cycle.get_slave_cycle_from_master_cycle(*cycle_to_sync);
                    if synced_cycle < emulator.get_cycles() {
                        panic!(
                            "msg from master: cycle problem {synced_cycle} < {}",
                            emulator.get_cycles()
                        );
                    }
                    if synced_cycle == emulator.get_cycles() {
                        let response = emulator
                            .serial
                            .set_msg_from_master(*value, &mut emulator.state);
                        if response != self.last_correction {
                            messages.push(
                                SerialMessage::FromSlave(cycle_to_sync.get_response(response))
                                    .serialize(),
                            );
                            self.messages_to_handle.clear();
                            self.last_correction = response;
                        }
                        self.messages_to_handle.pop_front();
                    }
                }
                MiamMessage::FromSlave(_, _) => {
                    panic!(
                        "shouldn't receive slave message because the master didn't even send messages to receive a response"
                    )
                }
            }
        }

        messages.extend(
            self.execute(emulator.serial.slave_byte, emulator.get_cycles())
                .into_iter()
                .map(|msg| SerialMessage::FromMaster(msg).serialize()),
        );

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

        messages
    }
}
