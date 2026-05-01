use crate::{
    apu::MAX_VOLUME,
    cells::{Dffr, DffrToggle, NegativeEdge, NorLatch, Tffnl},
};

#[derive(Clone, Default)]
struct EnvelopeTimer {
    value: u8, // 4 bits
    is_increasing: bool,
    sweep_pace: u8, // 3 bits
    pace_count: u8,
    stopped: bool,
}

impl EnvelopeTimer {
    fn trigger(&mut self, volume_and_envelope: u8) {
        self.is_increasing = volume_and_envelope & 0x08 != 0;
        self.value = (volume_and_envelope >> 4) & 0x0f;
        self.sweep_pace = volume_and_envelope & 0x07;
        self.pace_count = 0;
        self.stopped = false;
    }
    fn tick(&mut self) {
        // https://gbdev.io/pandocs/Audio_Registers.html#ff12--nr12-channel-1-volume--envelope
        // A setting of 0 disables the envelope.
        // about multiple of 8 https://gbdev.io/pandocs/Audio_details.html#div-apu
        if self.sweep_pace == 0 {
            return;
        }

        self.pace_count += 1;

        if self.pace_count != self.sweep_pace {
            return;
        }

        self.pace_count = 0;

        match (self.is_increasing, self.value) {
            (true, MAX_VOLUME) | (false, 0) => self.stopped = true,
            (true, _) => self.value += 1,
            (false, _) => self.value -= 1,
        }
    }
}

#[derive(Clone, Default)]
pub struct VolumeAndEnvelope {
    timer: EnvelopeTimer,
    register: u8,
}

impl VolumeAndEnvelope {
    pub fn is_dac_on(&self) -> bool {
        // https://gbdev.io/pandocs/Audio_details.html#dacs
        self.register & 0xf8 != 0
    }

    pub fn get_volume(&self) -> u8 {
        self.timer.value
    }

    pub fn get_register(&self) -> u8 {
        self.register
    }

    pub fn write_register(&mut self, value: u8) {
        self.zombie_mode_glitch(value);
        self.register = value;
    }

    // https://gbdev.io/pandocs/Audio_details.html#obscure-behavior
    // useful for Prehistorik Man intro
    fn zombie_mode_glitch(&mut self, value: u8) {
        let old_pace = self.register & 0x07;

        let is_increasing = self.register & 0x08 != 0;

        if old_pace == 0 && !self.timer.stopped {
            self.timer.value = self.timer.value.wrapping_add(1);
        } else if !is_increasing {
            self.timer.value = self.timer.value.wrapping_add(2);
        }

        let will_increase = value & 0x08 != 0;

        if is_increasing != will_increase {
            self.timer.value = 16u8.wrapping_sub(self.timer.value);
        }

        self.timer.value &= 0x0f;
    }

    pub fn trigger(&mut self) {
        self.timer.trigger(self.register);
    }

    pub fn tick(&mut self) {
        self.timer.tick();
    }
}

// ch2_eg_stop ch2_restart,jyro,ch2_env0,ch2_env1,ch2_env2,ch2_env3,ff17_d3,jopa
struct EnvelopeIsStopped {
    dffr: Dffr,
    nor_latch: NorLatch,
}

impl EnvelopeIsStopped {
    fn update(
        &mut self,
        channel_restart: bool,
        channel_env: SmallByte<4>,
        is_increasing: bool,
        apu_reset: bool,
        // jopa for ch2
        pace_finished_synced: bool,
    ) -> bool {
        let should_reset = apu_reset || channel_restart;
        let should_stop =
            !is_increasing && channel_env.get() == 0 || is_increasing && channel_env.get() == 0x0f;

        let is_stopped = self
            .dffr
            .update(should_stop, pace_finished_synced, !should_reset);

        self.nor_latch.update(is_stopped, should_reset)
    }
}

// kyvo horu_512hz,byfe_128hz,apu_reset,ch2_restart,jopa,ff17_d0_n,ff17_d1_n,ff17_d2_n
struct PaceIsFinished {
    clock_divider: DffrToggle,
    pace_counter: SmallByte<3>,
    counter_clk_edge: NegativeEdge,
}

impl PaceIsFinished {
    fn update(
        &mut self,
        clock_128hz: bool,
        apu_reset: bool,
        ch2_restart: bool,
        // jopa for ch2
        pace_finished_synced: bool,
        pace_reg: SmallByte<3>,
    ) -> bool {
        let should_load = ch2_restart || pace_finished_synced;

        let counter_clk = self.clock_divider.update(!clock_128hz, !apu_reset);

        if should_load {
            self.pace_counter = pace_reg
        }

        if self.counter_clk_edge.update(counter_clk) {
            self.pace_counter = self.pace_counter.increment();
        }

        self.pace_counter.get() == 0x07
    }
}

// jopa kyvo,horu_512hz,ff17_d0,ff17_d1,ff17_d2,ch2_restart,apu_reset
struct PaceIsFinishedSynced {
    syncer_512hz: Dffr,
    pace_is_finished: PaceIsFinished,
}

impl PaceIsFinishedSynced {
    fn update(
        &mut self,
        clock_512hz: bool,
        clock_128hz: bool,
        apu_reset: bool,
        ch2_restart: bool,
        pace_reg: SmallByte<3>,
    ) -> bool {
        let pace_is_finished = self.pace_is_finished.update(
            clock_128hz,
            apu_reset,
            ch2_restart,
            self.syncer_512hz.state,
            pace_reg,
        );

        let should_reset = !clock_512hz && self.syncer_512hz.state
            || pace_reg.get() == 0
            || ch2_restart
            || apu_reset;

        self.syncer_512hz
            .update(pace_is_finished, clock_512hz, !should_reset)
    }
}

// ch2_env0 ff17_d0,ff17_d1,ff17_d2,ff17_d3,ff17_d3_n,ff17_d4,ff17_d5,ff17_d6,ff17_d7,jopa,ch2_eg_stop,ch2_restart
struct EnvelopeValue {
    // I can't do a pretty increase/decrease logic with a normal byte because we have to emulate the zombie mode
    // glitch
    state: Tffnl,
}

impl EnvelopeValue {
    fn update(
        &mut self,
        pace_reg: SmallByte<3>,
        pace_finished_synced: bool,
        is_envelope_stopped: bool,
        is_increasing: bool,
        ch2_restart: bool,
        volume_reg: SmallByte<4>,
    ) {
        let ch2_env0 = self.state.update(
            0,
            volume_reg.get() & 1 != 0,
            ch2_restart,
            pace_finished_synced || pace_reg.get() == 0 || is_envelope_stopped,
        );

        let ch2_env1 = self.state.update(
            1,
            volume_reg.get() & 0b10 != 0,
            ch2_restart,
            is_increasing == ch2_env0,
        );

        let ch2_env2 = self.state.update(
            2,
            volume_reg.get() & 0b100 != 0,
            ch2_restart,
            is_increasing == ch2_env1,
        );

        self.state.update(
            3,
            volume_reg.get() & 0b1000 != 0,
            ch2_restart,
            is_increasing == ch2_env2,
        );
    }

    fn get_value(&self) -> SmallByte<4> {
        SmallByte(self.state.get_state())
    }
}

#[derive(Clone, Copy)]
struct SmallByte<const SIZE: u8>(u8);

impl<const SIZE: u8> SmallByte<SIZE> {
    fn get(self) -> u8 {
        self.0 & ((1 << SIZE) - 1)
    }
    fn increment(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
    fn decrement(self) -> Self {
        Self(self.0.wrapping_sub(1))
    }
}

struct EnvelopeComponent {
    register: u8,
    envelope_value: EnvelopeValue,
    pace_is_finished_synced: PaceIsFinishedSynced,
    envelope_is_stopped: EnvelopeIsStopped,
}

impl EnvelopeComponent {
    fn update(&mut self, clock_512hz: bool, clock_128hz: bool, apu_reset: bool, ch2_restart: bool) {
        let pace_is_finished_synced = self.pace_is_finished_synced.update(
            clock_512hz,
            clock_128hz,
            apu_reset,
            ch2_restart,
            SmallByte(self.register),
        );
        let is_increasing = self.register & 0b1000 != 0;
        let is_envelope_stopped = self.envelope_is_stopped.update(
            ch2_restart,
            self.envelope_value.get_value(),
            is_increasing,
            apu_reset,
            pace_is_finished_synced,
        );
        self.envelope_value.update(
            SmallByte(self.register),
            pace_is_finished_synced,
            is_envelope_stopped,
            is_increasing,
            ch2_restart,
            SmallByte(self.register >> 4),
        );
    }
}
