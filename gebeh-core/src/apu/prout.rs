use crate::{
    apu::envelope::EnvelopeComponent,
    cells::{Dffr, DffrToggle, DrlatchEe, NorLatch},
};

struct ChannelRestart {
    syncer_1mhz: Dffr,
    has_started: NorLatch,
    syncer_1mhz_has_started: Dffr,
}

impl ChannelRestart {
    pub fn update(&mut self, channel_1mhz: bool, apu_reset: bool, is_channel_starting: bool) {
        let is_restarting_synced =
            self.syncer_1mhz
                .update(self.syncer_1mhz_has_started.state, channel_1mhz, !apu_reset);

        let should_reset = apu_reset || is_restarting_synced;

        let has_started = !self.has_started.update(should_reset, is_channel_starting);
        self.syncer_1mhz_has_started
            .update(has_started, channel_1mhz, !should_reset);
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

struct Channel {
    apu_phi: ApuPhi,
    channel_1mhz: Channel1Mhz,
    channel_start: ChannelStart,
    chanel_restart: ChannelRestart,
    envelope: EnvelopeComponent,
    apu_reset: ApuReset,
    apu_clocks: ApuClocks,
}

impl Channel {
    fn update(
        &mut self,
        apu_wr: bool,
        ff26: bool,
        is_audio_on: bool,
        apu_4mhz: bool,
        is_triggering: bool,
        is_writing_to_nrx4: bool,
        system_clock: u16,
    ) {
        let apu_reset = self.apu_reset.update(false, apu_wr, ff26, is_audio_on);
        let channel_1mhz = self.channel_1mhz.update(apu_reset, apu_4mhz);
        let apu_phi = self.apu_phi.update(apu_4mhz);
        self.channel_start
            .update(apu_reset, is_triggering, is_writing_to_nrx4, apu_phi);
        self.chanel_restart
            .update(channel_1mhz, apu_reset, self.channel_start.get_state());
        self.apu_clocks
            .update(apu_4mhz, apu_reset, system_clock & (1 << 10) == 0);
        self.envelope.update(
            self.apu_clocks.get_horu_512hz(),
            self.apu_clocks.get_byfe_128hz(),
            apu_reset,
            self.chanel_restart.get_state(),
        );
    }
}

struct ApuClocks {
    ajer_inst: DffrToggle,
    bara_inst: Dffr,
    caru_inst: DffrToggle,
    bylu_inst: DffrToggle,
}

impl ApuClocks {
    fn update(&mut self, apu_4mhz: bool, apu_reset: bool, clock_512hz: bool) {
        let ajer_inst_output = self.ajer_inst.update(!apu_4mhz, !apu_reset);
        let ajer = ajer_inst_output;

        let bara_inst_output = self.bara_inst.update(clock_512hz, !ajer, !apu_reset);
        let bara_n = !bara_inst_output;

        let caru_inst_output = self.caru_inst.update(!bara_n, !apu_reset);
        let caru_n = !caru_inst_output;

        self.bylu_inst.update(caru_n, !apu_reset);
    }

    fn get_horu_512hz(&self) -> bool {
        !self.bara_inst.state
    }

    fn get_byfe_128hz(&self) -> bool {
        self.bylu_inst.state
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
                true, true, true, true, false, false, false, false, true, true, true, true, false,
                false, false, false,
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
