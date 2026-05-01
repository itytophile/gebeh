use crate::cells::{Dffr, NorLatch};

struct ChannelRestart {
    syncer_1mhz: Dffr,
    has_started: NorLatch,
    syncer_1mhz_has_started: Dffr,
}

impl ChannelRestart {
    pub fn update(&mut self, ch2_1mhz: bool, apu_reset: bool, ch2_start: bool) {
        let is_restarting_synced =
            self.syncer_1mhz
                .update(self.syncer_1mhz_has_started.state, ch2_1mhz, !apu_reset);

        let should_reset = apu_reset || is_restarting_synced;

        let has_started = !self.has_started.update(should_reset, ch2_start);
        self.syncer_1mhz_has_started
            .update(has_started, ch2_1mhz, !should_reset);
    }

    pub fn get_state(&self) -> bool {
        self.syncer_1mhz_has_started.state
    }
}
