use crate::cells::{Dffr, DffrToggle, DrlatchEe, NorLatch};

struct ChannelRestart {
    syncer_1mhz: Dffr,
    has_started: NorLatch,
    syncer_1mhz_has_started: Dffr,
}

impl ChannelRestart {
    pub fn update(&mut self, ch2_1mhz: bool, apu_reset: bool, is_channel_starting: bool) {
        let is_restarting_synced =
            self.syncer_1mhz
                .update(self.syncer_1mhz_has_started.state, ch2_1mhz, !apu_reset);

        let should_reset = apu_reset || is_restarting_synced;

        let has_started = !self.has_started.update(should_reset, is_channel_starting);
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

// apu_phi test_reset_n
// alef test_reset_n,adyk,apuk
// I don't really understand the "sort of" clock divider with drlatch_ee so let's brute copy the thing
// PS: according to the test below, it's dividing the clock by 4
#[derive(Default)]
struct ApuPhi {
    adyk_inst: DrlatchEe,
    afur_inst: DrlatchEe,
    alef_inst: DrlatchEe,
    apuk_inst: DrlatchEe,
}

impl ApuPhi {
    fn update(&mut self, apu_4mhz: bool) -> bool {
        let adyk_n = !self
            .adyk_inst
            .update(self.apuk_inst.get_state(), apu_4mhz, true);

        let afur = self.afur_inst.update(adyk_n, !apu_4mhz, true);

        let alef = self.alef_inst.update(afur, apu_4mhz, true);

        self.apuk_inst.update(alef, !apu_4mhz, true);

        afur
    }
}

#[derive(Default)]
struct CpuWr {
    adyk_inst: DrlatchEe,
    afur_inst: DrlatchEe,
    alef_inst: DrlatchEe,
    apuk_inst: DrlatchEe,
}

impl CpuWr {
    fn update(&mut self, avet: bool, write: bool) -> bool {
        let adyk = self
            .adyk_inst
            .update(self.apuk_inst.get_state(), avet, true);

        let afur = self.afur_inst.update(!adyk, !avet, true);

        let alef = self.alef_inst.update(afur, avet, true);

        self.apuk_inst.update(alef, !avet, true);

        adyk && !afur && write
    }
}

struct Channel1Mhz {
    first_divider: DffrToggle,
    second_divider: DffrToggle,
}

impl Channel1Mhz {
    fn update(&mut self, apu_reset: bool, apu_4mhz: bool) -> bool {
        let divided = self.first_divider.update(!apu_4mhz, !apu_reset);
        self.second_divider.update(!divided, !apu_reset)
    }
}

struct ApuReset {
    is_audio_on: Dffr,
}

impl ApuReset {
    fn update(&mut self, reset: bool, apu_wr: bool, ff26: bool, is_audio_on: bool) -> bool {
        let is_audio_on = self
            .is_audio_on
            .update(is_audio_on, !(ff26 && apu_wr), !reset);

        !is_audio_on || reset
    }
}

#[cfg(test)]
mod tests {
    use crate::apu::prout::{ApuPhi, CpuWr};
    extern crate std;

    #[test]
    fn apu_phi() {
        let mut apu_phi = ApuPhi::default();

        let mut apu_4mhz = false;

        let apu_phi_wave: std::vec::Vec<_> = (0..16)
            .map(|_| {
                let apu_phi = apu_phi.update(apu_4mhz);
                apu_4mhz = !apu_4mhz;
                apu_phi
            })
            .collect();

        assert_eq!(
            &[
                false, false, false, false, true, true, true, true, false, false, false, false,
                true, true, true, true
            ],
            apu_phi_wave.as_slice()
        )
    }

    #[test]
    fn cpu_wr() {
        let mut apu_phi = CpuWr::default();

        let mut avet = false;

        let apu_phi_wave: std::vec::Vec<_> = (0..31)
            .map(|_| {
                let apu_phi = apu_phi.update(avet, true);
                avet = !avet;
                apu_phi
            })
            .collect();

        assert_eq!(
            &[
                false, false, false, false, true, true, true, false, false, false, false, false,
                true, true, true, false, false, false, false, false, true, true, true, false,
                false, false, false, false, true, true, true
            ],
            apu_phi_wave.as_slice()
        )
    }
}
