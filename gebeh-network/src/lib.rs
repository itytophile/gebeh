use std::collections::VecDeque;

use arrayvec::ArrayVec;
use gebeh_core::{Emulator, EmulatorExt, Model, serial::Serial};
use gebeh_front_helper::{CloneMbc, EasyMbc};

use crate::{
    message::SerialMessage,
    synchro_cycles::{CycleToSync, SynchroCycles},
};

pub mod message;
mod synchro_cycles;

type Snapshots<M> = VecDeque<Snapshot<M>>;

type Snapshot<M> = (Emulator<M>, EasyMbc);

// 3 seconds
const ROLLBACK_TRESHOLD: u64 = 4194304 * 3 / 4;
// 10 ms

const MAX_SNAPSHOT: usize = (ROLLBACK_TRESHOLD / ROLLBACK_SNAPSHOT_PERIOD) as usize;
// half of a quantum batch duration
const ROLLBACK_SNAPSHOT_PERIOD: u64 = 4194304 / 4 * 128 / 48000 / 2;

enum MiamMessage {
    FromMaster(CycleToSync, u8),
    FromSlave(u64, u8),
}

pub struct RollbackSerial<M: Model> {
    master_snapshots: Snapshots<M>,
    slave_snapshots: Snapshots<M>,
    synchro_cycles: Option<SynchroCycles>,
    last_correction: u8,
    messages_to_handle: VecDeque<MiamMessage>,
    force_snapshot: bool,
}

impl<M: Model> Default for RollbackSerial<M> {
    fn default() -> Self {
        Self {
            master_snapshots: Default::default(),
            slave_snapshots: Default::default(),
            synchro_cycles: Default::default(),
            last_correction: 0xff,
            messages_to_handle: Default::default(),
            force_snapshot: false,
        }
    }
}

impl<M: Model> RollbackSerial<M> {
    pub fn add_messages(&mut self, msg: &[u8]) {
        let msg = SerialMessage::deserialize(msg);
        self.messages_to_handle.extend(
            msg.get()
                .iter()
                .filter(|msg| {
                    log::info!(
                        "Filtrage de {:?} 0x{:02x} -> {}",
                        (msg.is_master, msg.cycle, msg.prediction),
                        msg.value,
                        msg.prediction == self.last_correction
                    );
                    msg.prediction == self.last_correction
                })
                .map(|msg| {
                    if msg.is_master {
                        MiamMessage::FromMaster(CycleToSync::new(msg.cycle.to_native()), msg.value)
                    } else {
                        MiamMessage::FromSlave(msg.cycle.to_native(), msg.value)
                    }
                }),
        );
    }

    fn add_snapshot(&mut self, snapshot: Snapshot<M>) {
        if self.slave_snapshots.len() == MAX_SNAPSHOT {
            self.slave_snapshots.pop_front();
        }
        self.slave_snapshots.push_back(snapshot)
    }

    // never try to "catch up" when there is a rollback, that's too hard for phone CPUs

    pub fn rollback_if_necessary(&mut self, emulator: &mut Emulator<M>, mbc: &mut EasyMbc) {
        let Some(msg) = self.messages_to_handle.front() else {
            return;
        };

        let current_cycle = emulator.get_cycles();

        let (cycle, byte) = match msg {
            MiamMessage::FromMaster(cycle, value) => (*cycle, *value),
            MiamMessage::FromSlave(cycle, value) => {
                let synchro_cycle = self
                    .synchro_cycles.as_ref()
                    .unwrap();
                let cycle =
                    synchro_cycle.get_slave_cycle_from_master_cycle(CycleToSync::new(*cycle));
                log::info!("je rollback après msg du slave (g reçu 0x{value:02x})");
                let (mut snap_emulator, snap_mbc) = self
                    .master_snapshots
                    .drain(..)
                    .find(|(emulator, _)| emulator.get_cycles() == cycle)
                    .unwrap_or_else(|| {
                        panic!(
                            "desync too big, can't find cycle {cycle} (on cycle {})",
                            emulator.get_cycles()
                        )
                    });
                self.slave_snapshots.clear();
                snap_emulator.set_joypad(*emulator.get_joypad());
                *emulator = snap_emulator;
                *mbc = snap_mbc;

                emulator.serial.set_slave_byte(*value);
                // to avoid the master to reemit a message already handled by the slave
                emulator.execute(mbc.as_mut());

                self.messages_to_handle.pop_front();

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

        self.master_snapshots.clear();

        emulator.set_joypad(previous_input);
    }

    #[must_use]
    pub fn execute_and_take_snapshot(
        &mut self,
        emulator: &mut Emulator<M>,
        mbc: &mut dyn CloneMbc<'static>,
    ) -> ArrayVec<SerialMessage, 2> {
        if emulator
            .get_cycles()
            .is_multiple_of(ROLLBACK_SNAPSHOT_PERIOD)
            || self.force_snapshot
        {
            self.add_snapshot((emulator.clone(), mbc.clone_boxed()));
            self.force_snapshot = false;
        }

        let mut messages = ArrayVec::new();

        if let Some(msg) = self.messages_to_handle.front() {
            match msg {
                MiamMessage::FromMaster(cycle_to_sync, value) => {
                    if self.synchro_cycles.is_none() {
                        log::info!("j'init synchro cycle");
                    }
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
                        log::info!("Je reçois 0x{value:02x} de la part du master");
                        let response = emulator
                            .serial
                            .set_msg_from_master(*value, &mut emulator.interrupts);
                        self.force_snapshot = true;
                        if response != self.last_correction {
                            log::info!(
                                "J'envoie une correction au master 0x{:02x} -> 0x{:02x} avec mon synchro cycle à {:?}",
                                self.last_correction,
                                response,
                                synchro_cycle
                            );
                            messages.push(
                                cycle_to_sync
                                    .get_response(response, emulator.serial.get_slave_byte()),
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

        if emulator.will_serial_emit_byte() {
            let emulator_clone = emulator.clone();
            let mbc_clone = mbc.clone_boxed();
            let byte = emulator.execute(mbc).unwrap();
            self.master_snapshots.pop_front_if(|(snap, _)| {
                emulator.get_cycles() - snap.get_cycles() > ROLLBACK_TRESHOLD
            });
            log::info!("Moi master, j'envoie 0x{byte:02x}");
            let cycle_to_sync = CycleToSync::new(emulator_clone.get_cycles());
            if self.synchro_cycles.is_none() {
                log::info!("j'init synchro cyles avant l'envoie en tant que master");
            }
            // If the master is initializing its synchro cycle, then the synchro cycle will be equal to
            // 0 and that's good. It means that the "network" clock used by the slave and the master is equal to the
            // first master (in this session) clock.
            let synchro_cycle = self.synchro_cycles.get_or_insert(SynchroCycles::new(
                cycle_to_sync,
                emulator_clone.get_cycles(),
            ));
            let synced_cycle = synchro_cycle.get_master_cycle_from_master_cycle(cycle_to_sync);
            log::info!(
                "mon synchro cycle: {:?} {} -> {}",
                synchro_cycle,
                emulator_clone.get_cycles(),
                synced_cycle
            );
            messages.push(SerialMessage {
                is_master: true,
                value: byte,
                cycle: synced_cycle,
                prediction: emulator_clone.serial.get_slave_byte(),
            });
            self.master_snapshots.push_back((emulator_clone, mbc_clone));
        } else {
            emulator.execute(mbc);
        }

        messages
    }
}

pub fn handle_msg_no_emulator(msg: &[u8]) -> Option<SerialMessage> {
    let msg = SerialMessage::deserialize(msg);
    msg.get()
        .iter()
        .find(|msg| msg.is_master && msg.prediction != 0xff)
        .map(|msg| SerialMessage {
            is_master: false,
            prediction: 0xff,
            value: 0xff,
            cycle: msg.cycle.to_native(),
        })
}
