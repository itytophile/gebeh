use crate::apu::{
    noise_channel::NoiseChannel, pulse_channel::PulseChannel, sweep::Ch1Sweep,
    wave_channel::WaveChannel,
};

mod envelope;
mod length;
mod noise_channel;
mod pulse_channel;
mod sweep;
mod wave_channel;

#[derive(Clone, Default)]
pub struct Apu {
    is_on: bool,
    nr51: Nr51,
    nr50: Nr50,
    pub ch1: PulseChannel<Ch1Sweep>,
    pub ch2: PulseChannel<()>,
    pub ch3: WaveChannel,
    pub ch4: NoiseChannel,
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
    pub fn get_nr52(&self) -> u8 {
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
            *self = Default::default();
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

    pub fn execute(&mut self, div: u8) {
        if !self.is_on {
            return;
        }
        self.ch1.tick(div);
        self.ch2.tick(div);
        self.ch3.tick(div);
        self.ch4.tick(div);
    }

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
