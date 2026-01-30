use crate::apu::{
    MAX_VOLUME,
    length::{Length, MASK_8_BITS},
};

#[derive(Default, Clone)]
pub struct WaveChannel {
    is_enabled: bool,
    is_dac_on: bool,
    length: Length<MASK_8_BITS>,
    output_level: u8, // 2 bits
    effective_output_level: u8,
    period: u16, // 11 bits
    ram: [u8; 16],
}

impl WaveChannel {
    pub fn tick_length(&mut self) {
        self.is_enabled &= !self.length.tick();
    }
    pub fn get_nr30(&self) -> u8 {
        ((self.is_dac_on as u8) << 7) | 0b01111111
    }
    pub fn write_nr30(&mut self, value: u8) {
        self.is_dac_on = value & 0x80 != 0;
        self.is_enabled &= self.is_dac_on;
    }
    pub fn get_nr31(&self) -> u8 {
        0xff
    }
    pub fn write_nr31(&mut self, value: u8) {
        self.length.set_initial_timer_length(value);
    }
    pub fn get_nr32(&self) -> u8 {
        (self.output_level << 5) | 0b10011111
    }
    pub fn write_nr32(&mut self, value: u8) {
        self.output_level = (value >> 5) & 0b11;
    }
    pub fn get_nr33(&self) -> u8 {
        0xff
    }
    pub fn write_nr33(&mut self, value: u8) {
        self.period = self.period & 0xff00 | u16::from(value);
    }
    pub fn get_nr34(&self) -> u8 {
        ((self.length.is_enabled() as u8) << 6) | 0b10111111
    }
    pub fn write_nr34(&mut self, value: u8, div_apu: u8) {
        self.period = (u16::from(value & 0x07) << 8) | self.period & 0x00ff;
        self.is_enabled &= !self.length.set_is_enabled(value & 0x40 != 0, div_apu);
        if value & 0x80 != 0 {
            self.trigger(div_apu);
        }
    }
    fn trigger(&mut self, div_apu: u8) {
        // according to blargg "Disabled DAC shouldn't stop other trigger effects"
        self.length.trigger(div_apu);

        // according to blargg "Disabled DAC should prevent enable at trigger"
        if !self.is_dac_on {
            return;
        }
        self.is_enabled = true;
        self.effective_output_level = self.output_level;
    }
    pub fn is_on(&self) -> bool {
        self.is_enabled
    }
    // let's ignore specific behaviors
    // https://gbdev.io/pandocs/Audio_Registers.html#ff30ff3f--wave-pattern-ram
    pub fn write_ram(&mut self, index: u8, value: u8) {
        if self.is_on() {
            return;
        }
        self.ram[usize::from(index)] = value;
    }
    pub fn read_ram(&self, index: u8) -> u8 {
        if self.is_on() {
            return 0xff;
        }
        self.ram[usize::from(index)]
    }

    pub fn get_sampler(&self) -> WaveSampler {
        WaveSampler {
            is_on: self.is_on(),
            effective_output_level: self.effective_output_level,
            ram: self.ram,
            period: self.period,
            is_dac_on: self.is_dac_on,
            sample_shift: 0.,
        }
    }

    #[must_use]
    pub fn reset(&self) -> Self {
        Self {
            length: self.length.reset(),
            ram: self.ram,
            ..Default::default()
        }
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct WaveSampler {
    is_on: bool,
    effective_output_level: u8,
    ram: [u8; 16],
    pub period: u16,
    is_dac_on: bool,
    pub sample_shift: f32,
}

impl WaveSampler {
    pub fn sample(&self, sample: f32) -> f32 {
        // https://gbdev.io/pandocs/Audio_details.html#channels
        // Citation: a disabled channel outputs 0, which an enabled DAC will dutifully convert into “analog 1”.
        if !self.is_dac_on {
            return 0.;
        }
        // About output level https://gbdev.io/pandocs/Audio_Registers.html#ff1c--nr32-channel-3-output-level
        if !self.is_on || self.effective_output_level == 0 {
            return 1.;
        }

        1. - (index_ram(
            &self.ram,
            ((((sample - self.sample_shift) * Self::get_tone_frequency(self.period)) % 1.) * 32.)
                as usize,
        ) as f32) as f32
            / MAX_VOLUME as f32
            * 2.
    }

    // https://gbdev.io/pandocs/Audio_Registers.html#ff1d--nr33-channel-3-period-low-write-only
    pub fn get_tone_frequency(period: u16) -> f32 {
        65536. / (2048. - period as f32)
    }
}

fn index_ram(ram: &[u8; 16], index: usize) -> u8 {
    let two_samples = ram[index / 2];

    // https://gbdev.io/pandocs/Audio_Registers.html#ff30ff3f--wave-pattern-ram
    // Citation: As CH3 plays, it reads wave RAM left to right, upper nibble first
    if index.is_multiple_of(2) {
        two_samples >> 4
    } else {
        two_samples & 0x0f
    }
}
