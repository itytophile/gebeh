use crate::apu::{
    envelope::VolumeAndEnvelope,
    length::{Length, MASK_6_BITS},
    sweep::{Ch1Sweep, Sweep},
};

type Wave = [f32; 8];

// https://gbdev.io/pandocs/Audio_Registers.html#ff11--nr11-channel-1-length-timer--duty-cycle
const WAVE_00: Wave = [1., 1., 1., 1., 1., 1., 1., -1.];
const WAVE_01: Wave = [-1., 1., 1., 1., 1., 1., 1., -1.];
const WAVE_10: Wave = [-1., 1., 1., 1., 1., -1., -1., -1.];
const WAVE_11: Wave = [1., -1., -1., -1., -1., -1., -1., 1.];

#[derive(Clone, Default)]
pub struct PulseChannel<S: Sweep> {
    length: Length<MASK_6_BITS>,
    duty_cycle: u8,
    volume_and_envelope: VolumeAndEnvelope,
    period_low: u8,
    period_high: u8,
    is_enabled: bool,
    sweep: S,
}

impl<S: Sweep> PulseChannel<S> {
    pub fn tick_envelope(&mut self) {
        if self.is_on() {
            self.volume_and_envelope.tick();
        }
    }
    pub fn tick_length(&mut self, cycles: u64, ch: &'static str) {
        self.is_enabled &= !self.length.tick(cycles, ch);
    }
    pub fn get_nrx1(&self) -> u8 {
        (self.duty_cycle << 6) | 0b00111111
    }
    pub fn write_nrx1(&mut self, value: u8) {
        self.duty_cycle = value >> 6;
        self.length.set_initial_timer_length(value);
    }
    pub fn get_nrx2(&self) -> u8 {
        self.volume_and_envelope.get_register()
    }
    pub fn write_nrx2(&mut self, value: u8) {
        self.volume_and_envelope.write_register(value);
        self.is_enabled &= self.volume_and_envelope.is_dac_on();
    }
    pub fn get_nrx3(&self) -> u8 {
        0xff
    }
    pub fn write_nrx3(&mut self, value: u8) {
        self.period_low = value;
    }
    pub fn get_nrx4(&self) -> u8 {
        ((self.length.is_enable as u8) << 6) | 0b10111111
    }
    pub fn write_nrx4(&mut self, value: u8, ch: &'static str, div_apu: u8, cycles: u64) {
        if !self.length.is_enable && value & 0x40 != 0 {
            self.length.is_enable = true;
            log::info!("{ch} length enabled!");
            // for this hack to work, the cpu must be executed after the apu (I suppose)
            // according to blargg "Enabling in first half of length period should clock length"
            if div_apu.is_multiple_of(2) {
                log::info!("extra tick");
                self.tick_length(cycles, ch);
            }
        }
        if self.length.is_enable && value & 0x40 == 0 {
            log::info!("{ch} length disabled!")
        }
        self.length.is_enable = value & 0x40 != 0;

        self.period_high = value & 0x07;
        if value & 0x80 != 0 {
            self.trigger(ch, div_apu.is_multiple_of(2));
        }
    }

    pub fn trigger(&mut self, ch: &'static str, extra_clock: bool) {
        self.length.trigger(extra_clock);
        
        // according to blargg "Disabled DAC should prevent enable at trigger"
        if !self.volume_and_envelope.is_dac_on() {
            return;
        }
        
        if !self.is_enabled {
            log::info!("{ch} enabled!")
        }
        self.is_enabled = true;
        self.volume_and_envelope.trigger();
        if let Some(new_period) = self.sweep.trigger(self.get_period_value()) {
            self.set_period_value(new_period);
        }
    }

    pub fn is_on(&self) -> bool {
        self.is_enabled
    }

    pub fn tick_sweep(&mut self) {
        if !self.is_on() {
            return;
        }
        let (is_enabled_from_sweep, new_period) = self.sweep.tick();
        if let Some(period) = new_period {
            self.set_period_value(period);
        }
        self.is_enabled = is_enabled_from_sweep;
    }

    // 11 bits
    fn get_period_value(&self) -> u16 {
        u16::from_be_bytes([self.period_high & 0x07, self.period_low])
    }

    fn set_period_value(&mut self, value: u16) {
        self.period_low = value as u8;
        self.period_high = ((value >> 8) as u8) & 0x07;
    }

    pub fn get_sampler(&self) -> PulseSampler {
        PulseSampler {
            is_on: self.is_on(),
            duty_cycle: self.duty_cycle,
            period: self
                .sweep
                .get_period_value()
                .unwrap_or(self.get_period_value()),
            volume: self.volume_and_envelope.get_volume(),
        }
    }
}

impl PulseChannel<Ch1Sweep> {
    pub fn get_nr10(&self) -> u8 {
        self.sweep.nr10 | 0x80
    }
    pub fn write_nr10(&mut self, value: u8) {
        let new_pace = (value >> 4) & 0x07;

        // https://gbdev.io/pandocs/Audio_Registers.html#ff10--nr10-channel-1-sweep
        // Citation: However, if 0 is written to this field, then iterations are instantly
        // disabled, and it will be reloaded as soon as it’s set to something else.
        if new_pace == 0 {
            self.sweep.pace = new_pace;
        }

        if (self.sweep.nr10 >> 4) & 0x07 == 0 {
            self.sweep.pace = new_pace;
        }

        self.sweep.nr10 = value;
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct PulseSampler {
    is_on: bool,
    duty_cycle: u8,
    period: u16,
    volume: u8,
}

impl PulseSampler {
    pub fn sample(&self, sample: f32) -> f32 {
        if !self.is_on {
            return 0.;
        }
        // let space_size = sample_rate as f32 / self.get_tone_frequency();
        // let index_in_freq_space = index as f32 % space_size;
        // let normalized_index = index_in_freq_space / space_size;
        // Better function thanks to
        // (a % b) / b = (a / b) % 1.0
        let index = (sample * self.get_tone_frequency()) % 1.;
        let index = (index * 8.) as usize;
        let wave = match self.duty_cycle {
            0b00 => WAVE_00,
            0b01 => WAVE_01,
            0b10 => WAVE_10,
            0b11 => WAVE_11,
            _ => unreachable!(),
        };
        wave[index] * self.volume as f32 / 15.
    }
    // https://gbdev.io/pandocs/Audio_Registers.html#ff13--nr13-channel-1-period-low-write-only
    fn get_tone_frequency(&self) -> f32 {
        131072.0 / (2048.0 - self.period as f32)
    }
}
