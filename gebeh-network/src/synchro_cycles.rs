use std::fmt::Display;

use rkyv::{Archive, Deserialize, Serialize};

use crate::message::SerialMessage;

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct NetworkCycle(u64);

impl Display for NetworkCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl NetworkCycle {
    pub fn new(cycle: u64) -> Self {
        Self(cycle)
    }
    pub fn get_response(self, value: u8, prediction: u8) -> SerialMessage {
        SerialMessage {
            is_master: false,
            value,
            cycle: self,
            prediction,
        }
    }
}

#[derive(Debug, Default)]
pub struct SynchroCycles {
    diff: i64,
}

impl SynchroCycles {
    pub fn new(master: NetworkCycle, slave: u64) -> Self {
        Self {
            diff: i64::try_from(slave).unwrap() - i64::try_from(master.0).unwrap(),
        }
    }

    pub fn to_local_cycle(&self, cycle: NetworkCycle) -> u64 {
        cycle.0.strict_add_signed(self.diff)
    }

    pub fn to_network_cycle(&self, cycle: u64) -> NetworkCycle {
        NetworkCycle::new(cycle.strict_sub_signed(self.diff))
    }
}
