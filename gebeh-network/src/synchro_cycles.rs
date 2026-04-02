use crate::message::MessageFromSlave;

#[derive(Clone, Copy)]
pub struct CycleToSync(u64);

impl CycleToSync {
    pub fn new(cycle: u64) -> Self {
        Self(cycle)
    }
    pub fn get_response(self, value: u8) -> MessageFromSlave {
        MessageFromSlave {
            correction: value,
            cycle: self.0,
        }
    }
}

pub struct SynchroCycles {
    master: u64,
    slave: u64,
}

impl SynchroCycles {
    pub fn new(master: CycleToSync, slave: u64) -> Self {
        Self {
            master: master.0,
            slave,
        }
    }

    pub fn get_slave_cycle_from_master_cycle(&self, cycle: CycleToSync) -> u64 {
        self.slave + cycle.0 - self.master
    }
}
