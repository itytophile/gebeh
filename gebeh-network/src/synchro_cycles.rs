use std::fmt::Display;

use crate::message::SerialMessage;

#[derive(Clone, Copy)]
pub struct CycleToSync(u64);

impl Display for CycleToSync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl CycleToSync {
    pub fn new(cycle: u64) -> Self {
        Self(cycle)
    }
    pub fn get_response(self, value: u8, prediction: u8) -> SerialMessage {
        SerialMessage {
            is_master: false,
            value,
            cycle: self.0,
            prediction,
        }
    }
}

pub struct SynchroCycles {
    diff: i64,
}

impl SynchroCycles {
    pub fn new(master: CycleToSync, slave: u64) -> Self {
        Self {
            diff: i64::try_from(slave).unwrap() - i64::try_from(master.0).unwrap(),
        }
    }

    pub fn get_slave_cycle_from_master_cycle(&self, cycle: CycleToSync) -> u64 {
        cycle.0.strict_add_signed(self.diff)
    }
}
