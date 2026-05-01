use crate::cells::{Dffr, DrlatchEe, NorLatch};

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

// ch2_start apu_phi,apu_reset,ff19,apu_wr
// etap ch2_start,apu_phi,apu_reset,ff19,apu_wr
struct ChannelStart {
    is_starting: DrlatchEe,
    is_starting_synced: Dffr,
}

impl ChannelStart {
    fn update(
        &mut self,
        apu_reset: bool,
        is_triggering: bool,
        is_writing_to_nrx4: bool,
        apu_phi: bool,
    ) {
        let is_starting = self.is_starting.update(
            is_triggering,
            is_writing_to_nrx4,
            !(apu_reset || self.is_starting_synced.state),
        );
        self.is_starting_synced
            .update(is_starting, apu_phi, !apu_reset);
    }

    fn get_state(&self) -> bool {
        self.is_starting_synced.state
    }
}
