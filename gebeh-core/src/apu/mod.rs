use crate::apu::{
    noise_channel::{NoiseChannel, NoiseSampler},
    pulse_channel::{PulseChannel, PulseSampler},
    sweep::Ch1Sweep,
    wave_channel::{WaveChannel, WaveSampler},
};

mod envelope;
mod length;
mod noise_channel;
mod pulse_channel;
mod sweep;
mod wave_channel;

#[derive(Default, Clone)]
pub struct FallingEdge(bool);

impl FallingEdge {
    pub fn update(&mut self, value: bool) -> bool {
        let previous = self.0;
        self.0 = value;
        previous && !value
    }
}

#[derive(Clone, Default)]
pub struct Apu {
    is_on: bool,
    nr51: Nr51,
    nr50: Nr50,
    pub ch1: PulseChannel<Ch1Sweep>,
    pub ch2: PulseChannel<()>,
    pub ch3: WaveChannel,
    pub ch4: NoiseChannel,
    // https://gbdev.io/pandocs/Audio_details.html#div-apu
    div_apu: u8,
    falling_edge: FallingEdge,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq)]
    pub struct Nr52: u8 {
        const AUDIO_ON_OFF = 1 << 7;
        const CH4_ON = 1 << 3;
        const CH3_ON = 1 << 2;
        const CH2_ON = 1 << 1;
        const CH1_ON = 1;
    }
}

// Sound panning
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq, Default)]
    pub struct Nr51: u8 {
        const CH4_LEFT = 1 << 7;
        const CH3_LEFT = 1 << 6;
        const CH2_LEFT = 1 << 5;
        const CH1_LEFT = 1 << 4;
        const CH4_RIGHT = 1 << 3;
        const CH3_RIGHT = 1 << 2;
        const CH2_RIGHT = 1 << 1;
        const CH1_RIGHT = 1;
    }
}

// Master volume & VIN panning
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq, Default)]
    pub struct Nr50: u8 {
        const VIN_LEFT = 1 << 7;
        const LEFT_VOLUME_MASK = 0b01110000;
        const VIN_RIGHT = 1 << 3;
        const RIGHT_VOLUME_MASK = 0b00000111;
    }
}

impl Apu {
    pub fn increment_div_apu(&mut self) {
        self.div_apu = self.div_apu.wrapping_add(1);
    }
    pub fn get_nr52(&self, _: u64) -> u8 {
        let mut flags = Nr52::empty();
        flags.set(Nr52::AUDIO_ON_OFF, self.is_on);
        flags.set(Nr52::CH4_ON, self.ch4.is_on());
        flags.set(Nr52::CH3_ON, self.ch3.is_on());
        flags.set(Nr52::CH2_ON, self.ch2.is_on());
        flags.set(Nr52::CH1_ON, self.ch1.is_on());

        flags.bits() | 0b01110000
    }
    pub fn write_nr52(&mut self, value: u8) {
        let is_on = Nr52::from_bits_retain(value).contains(Nr52::AUDIO_ON_OFF);
        if self.is_on == is_on {
            return;
        }
        self.is_on = is_on;
        if !self.is_on {
            *self = Self {
                ch1: self.ch1.reset(),
                ch2: self.ch2.reset(),
                ch3: self.ch3.reset(),
                ch4: self.ch4.reset(),
                ..Default::default()
            }
        }
    }
    pub fn get_nr51(&self) -> u8 {
        self.nr51.bits()
    }
    pub fn write_nr51(&mut self, value: u8) {
        self.nr51 = Nr51::from_bits_retain(value);
    }
    pub fn get_nr50(&self) -> u8 {
        self.nr50.bits()
    }
    pub fn write_nr50(&mut self, value: u8) {
        self.nr50 = Nr50::from_bits_retain(value);
    }

    #[must_use]
    pub fn execute(&mut self, div: u8) -> bool {
        if !self.is_on || !self.falling_edge.update(div & (1 << 4) != 0) {
            return false;
        }

        // 512 Hz
        if self.div_apu.is_multiple_of(2) {
            self.ch1.tick_length();
            self.ch2.tick_length();
            self.ch3.tick_length();
            self.ch4.tick_length();
        }
        if self.div_apu % 4 == 2 {
            self.ch1.tick_sweep();
        }
        if self.div_apu % 8 == 7 {
            self.ch1.tick_envelope();
            self.ch2.tick_envelope();
            self.ch4.tick_envelope();
        }

        true
    }

    pub fn get_sampler(&self) -> Sampler {
        Sampler {
            ch1: self.ch1.get_sampler(),
            ch2: self.ch2.get_sampler(),
            ch3: self.ch3.get_sampler(),
            ch4: self.ch4.get_sampler(),
            nr50: self.nr50,
            nr51: self.nr51,
        }
    }

    pub fn read(&self, index: u16, cycles: u64) -> u8 {
        use crate::state::*;
        match index {
            CH1_SWEEP => self.ch1.get_nr10(),
            CH1_LENGTH_TIMER_AND_DUTY_CYCLE => self.ch1.get_nrx1(),
            CH1_VOLUME_AND_ENVELOPE => self.ch1.get_nrx2(),
            CH1_PERIOD_LOW => self.ch1.get_nrx3(),
            CH1_PERIOD_HIGH_AND_CONTROL => self.ch1.get_nrx4(),
            0xff15 => 0xff,
            CH2_LENGTH_TIMER_AND_DUTY_CYCLE => self.ch2.get_nrx1(),
            CH2_VOLUME_AND_ENVELOPE => self.ch2.get_nrx2(),
            CH2_PERIOD_LOW => self.ch2.get_nrx3(),
            CH2_PERIOD_HIGH_AND_CONTROL => self.ch2.get_nrx4(),
            CH3_DAC_ENABLE => self.ch3.get_nr30(),
            CH3_LENGTH_TIMER => self.ch3.get_nr31(),
            CH3_OUTPUT_LEVEL => self.ch3.get_nr32(),
            CH3_PERIOD_LOW => self.ch3.get_nr33(),
            CH3_PERIOD_HIGH_AND_CONTROL => self.ch3.get_nr34(),
            0xff1f => 0xff,
            CH4_LENGTH_TIMER => self.ch4.read_nr41(),
            CH4_VOLUME_AND_ENVELOPE => self.ch4.read_nr42(),
            CH4_FREQUENCY_AND_RANDOMNESS => self.ch4.read_nr43(),
            CH4_CONTROL => self.ch4.read_nr44(),
            MASTER_VOLUME_AND_VIN_PANNING => self.get_nr50(),
            SOUND_PANNING => self.get_nr51(),
            AUDIO_MASTER_CONTROL => self.get_nr52(cycles),
            0xff27..WAVE => 0xff,
            WAVE..LCD_CONTROL => self.ch3.read_ram(u8::try_from(index - WAVE).unwrap()),
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, index: u16, value: u8) {
        use crate::state::*;

        // according to blargg we can write to the initial length timer registers when the apu is off
        match (index, self.is_on) {
            (CH1_SWEEP, true) => self.ch1.write_nr10(value),
            (CH1_LENGTH_TIMER_AND_DUTY_CYCLE, _) => self.ch1.write_nrx1(value, self.is_on),
            (CH1_VOLUME_AND_ENVELOPE, true) => self.ch1.write_nrx2(value),
            (CH1_PERIOD_LOW, true) => self.ch1.write_nrx3(value),
            (CH1_PERIOD_HIGH_AND_CONTROL, true) => self.ch1.write_nrx4(value, self.div_apu),
            (CH2_LENGTH_TIMER_AND_DUTY_CYCLE, _) => self.ch2.write_nrx1(value, self.is_on),
            (CH2_VOLUME_AND_ENVELOPE, true) => self.ch2.write_nrx2(value),
            (CH2_PERIOD_LOW, true) => self.ch2.write_nrx3(value),
            (CH2_PERIOD_HIGH_AND_CONTROL, true) => self.ch2.write_nrx4(value, self.div_apu),
            (CH3_DAC_ENABLE, true) => self.ch3.write_nr30(value),
            (CH3_LENGTH_TIMER, _) => self.ch3.write_nr31(value),
            (CH3_OUTPUT_LEVEL, true) => self.ch3.write_nr32(value),
            (CH3_PERIOD_LOW, true) => self.ch3.write_nr33(value),
            (CH3_PERIOD_HIGH_AND_CONTROL, true) => self.ch3.write_nr34(value, self.div_apu),
            (CH4_LENGTH_TIMER, _) => self.ch4.write_nr41(value),
            (CH4_VOLUME_AND_ENVELOPE, true) => self.ch4.write_nr42(value),
            (CH4_FREQUENCY_AND_RANDOMNESS, true) => self.ch4.write_nr43(value),
            (CH4_CONTROL, true) => self.ch4.write_nr44(value, self.div_apu),
            (MASTER_VOLUME_AND_VIN_PANNING, true) => self.write_nr50(value),
            (SOUND_PANNING, true) => self.write_nr51(value),
            (AUDIO_MASTER_CONTROL, _) => self.write_nr52(value),
            (WAVE..LCD_CONTROL, _) => {
                self.ch3
                    .write_ram(u8::try_from(index - WAVE).unwrap(), value);
            }
            _ => {}
        }
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct Sampler {
    ch1: PulseSampler,
    ch2: PulseSampler,
    ch3: WaveSampler,
    ch4: NoiseSampler,
    nr51: Nr51,
    nr50: Nr50,
}

impl Sampler {
    #[must_use]
    pub fn sample_left(&self, sample: f32, noise: &[u8], short_noise: &[u8]) -> f32 {
        ((if self.nr51.contains(Nr51::CH1_LEFT) {
            self.ch1.sample(sample)
        } else {
            0.0
        }) + (if self.nr51.contains(Nr51::CH2_LEFT) {
            self.ch2.sample(sample)
        } else {
            0.
        }) + (if self.nr51.contains(Nr51::CH3_LEFT) {
            self.ch3.sample(sample)
        } else {
            0.
        }) + (if self.nr51.contains(Nr51::CH4_LEFT) {
            self.ch4.sample(sample, noise, short_noise)
        } else {
            0.
        })) * self.get_volume_left()
    }

    #[must_use]
    pub fn sample_right(&self, sample: f32, noise: &[u8], short_noise: &[u8]) -> f32 {
        ((if self.nr51.contains(Nr51::CH1_RIGHT) {
            self.ch1.sample(sample)
        } else {
            0.0
        }) + (if self.nr51.contains(Nr51::CH2_RIGHT) {
            self.ch2.sample(sample)
        } else {
            0.
        }) + (if self.nr51.contains(Nr51::CH3_RIGHT) {
            self.ch3.sample(sample)
        } else {
            0.
        }) + (if self.nr51.contains(Nr51::CH4_RIGHT) {
            self.ch4.sample(sample, noise, short_noise)
        } else {
            0.
        })) * self.get_volume_right()
    }

    fn get_volume_left(&self) -> f32 {
        (((self.nr50.bits() >> 4) & 0x7) + 1) as f32 / 8.
    }

    fn get_volume_right(&self) -> f32 {
        ((self.nr50.bits() & 0x7) + 1) as f32 / 8.
    }
}
