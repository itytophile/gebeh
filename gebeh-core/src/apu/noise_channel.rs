use crate::apu::{
    MAX_VOLUME,
    envelope::VolumeAndEnvelope,
    length::{Length, MASK_6_BITS},
};

#[derive(Default, Clone)]
pub struct NoiseChannel {
    length: Length<MASK_6_BITS>,
    volume_and_envelope: VolumeAndEnvelope,
    nr43: u8,
    is_enabled: bool,
}

impl NoiseChannel {
    #[must_use]
    pub fn reset(&self) -> Self {
        Self {
            length: self.length.reset(),
            ..Default::default()
        }
    }
    pub fn tick_envelope(&mut self) {
        if self.is_on() {
            self.volume_and_envelope.tick();
        }
    }
    pub fn tick_length(&mut self) {
        self.is_enabled &= !self.length.tick();
    }
    pub fn write_nr41(&mut self, value: u8) {
        self.length.set_initial_timer_length(value);
    }
    pub fn read_nr41(&self) -> u8 {
        0xff
    }
    pub fn write_nr42(&mut self, value: u8) {
        self.volume_and_envelope.write_register(value);
        self.is_enabled &= self.volume_and_envelope.is_dac_on();
    }
    pub fn read_nr42(&self) -> u8 {
        self.volume_and_envelope.get_register()
    }
    pub fn write_nr43(&mut self, value: u8) {
        self.nr43 = value;
    }
    pub fn read_nr43(&self) -> u8 {
        self.nr43
    }
    pub fn write_nr44(&mut self, value: u8, div_apu: u8) {
        self.is_enabled &= !self.length.set_is_enabled(value & 0x40 != 0, div_apu);
        if value & 0x80 != 0 {
            self.trigger(div_apu);
        }
    }
    pub fn read_nr44(&self) -> u8 {
        ((self.length.is_enabled() as u8) << 6) | 0b10111111
    }

    fn trigger(&mut self, div_apu: u8) {
        // according to blargg "Disabled DAC shouldn't stop other trigger effects"
        self.length.trigger(div_apu);

        // according to blargg "Disabled DAC should prevent enable at trigger"
        if !self.volume_and_envelope.is_dac_on() {
            return;
        }
        self.is_enabled = true;
        self.volume_and_envelope.trigger();
    }

    pub fn is_on(&self) -> bool {
        self.volume_and_envelope.is_dac_on() && self.is_enabled
    }

    fn get_divider(&self) -> u8 {
        self.nr43 & 0x7
    }
    fn get_shift(&self) -> u8 {
        (self.nr43 >> 4) & 0xf
    }
    fn is_short_mode(&self) -> bool {
        self.nr43 & 0x8 != 0
    }

    pub fn get_sampler(&self) -> NoiseSampler {
        NoiseSampler {
            is_on: self.is_on(),
            divider: self.get_divider(),
            shift: self.get_shift(),
            is_short_mode: self.is_short_mode(),
            volume: self.volume_and_envelope.get_volume(),
            is_dac_on: self.volume_and_envelope.is_dac_on(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct NoiseSampler {
    is_on: bool,
    divider: u8,
    shift: u8,
    is_short_mode: bool,
    volume: u8,
    is_dac_on: bool,
}

impl NoiseSampler {
    pub fn sample(&self, sample: f32, noise: &[u8], short_noise: &[u8]) -> f32 {
        // https://gbdev.io/pandocs/Audio_details.html#channels
        // Citation: a disabled channel outputs 0, which an enabled DAC will dutifully convert into “analog 1”.
        if !self.is_dac_on {
            return 0.;
        }
        if !self.is_on {
            return 1.;
        }

        let freq = self.get_tick_frequency();
        // if freq is equal to A Hz then it means the lfsr has emitted A different values in 1 second.
        // The noise is cyclic so we can use modulo if the index is greater than the provided noise values.
        let index = (sample * freq) as usize;

        // 0 or 1
        let raw_sample = if self.is_short_mode {
            short_noise[index % short_noise.len()]
        } else {
            noise[index % noise.len()]
        };

        1. - (raw_sample * self.volume) as f32 / MAX_VOLUME as f32 * 2.
    }
    fn get_tick_frequency(&self) -> f32 {
        // https://gbdev.io/pandocs/Audio_Registers.html#ff22--nr43-channel-4-frequency--randomness
        // Citation: Note that divider = 0 is treated as divider = 0.5 instead.
        let divider: f32 = if self.divider == 0 {
            0.5
        } else {
            self.divider as f32
        };
        262144.0 / (divider * 2.0f32.powi(self.shift.into()))
    }
}
