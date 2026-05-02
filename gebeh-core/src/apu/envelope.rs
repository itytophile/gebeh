use crate::cells::{Dffr, DffrToggle, NegativeEdge, NorLatch, Tffnl};

// ch2_eg_stop ch2_restart,jyro,ch2_env0,ch2_env1,ch2_env2,ch2_env3,ff17_d3,jopa
#[derive(Default, Clone)]
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
#[derive(Default, Clone)]
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
#[derive(Default, Clone)]
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
#[derive(Default, Clone)]
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

#[derive(Clone, Copy, Default)]
pub struct SmallByte<const SIZE: u8>(u8);

impl<const SIZE: u8> SmallByte<SIZE> {
    pub fn get(self) -> u8 {
        self.0 & ((1 << SIZE) - 1)
    }
    pub fn increment(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
    pub fn decrement(self) -> Self {
        Self(self.0.wrapping_sub(1))
    }
}

#[derive(Default, Clone)]
pub struct EnvelopeComponent {
    register: u8,
    envelope_value: EnvelopeValue,
    pace_is_finished_synced: PaceIsFinishedSynced,
    envelope_is_stopped: EnvelopeIsStopped,
}

impl EnvelopeComponent {
    pub fn update(
        &mut self,
        horu_512hz: bool,
        byfe_128hz: bool,
        apu_reset: bool,
        channel_restart: bool,
    ) {
        let pace_is_finished_synced = self.pace_is_finished_synced.update(
            horu_512hz,
            byfe_128hz,
            apu_reset,
            channel_restart,
            SmallByte(self.register),
        );
        let is_increasing = self.register & 0b1000 != 0;
        let is_envelope_stopped = self.envelope_is_stopped.update(
            channel_restart,
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
            channel_restart,
            SmallByte(self.register >> 4),
        );
    }

    pub fn is_dac_on(&self) -> bool {
        // https://gbdev.io/pandocs/Audio_details.html#dacs
        self.register & 0xf8 != 0
    }

    pub fn get_volume(&self) -> u8 {
        self.envelope_value.get_value().0
    }

    pub fn get_register(&self) -> u8 {
        self.register
    }

    pub fn write_register(&mut self, value: u8) {
        self.register = value;
    }
}
