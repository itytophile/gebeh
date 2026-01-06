use crate::apu::{
    envelope::VolumeAndEnvelope, length::Length, pulse_channel::PulseChannel, sweep::Ch1Sweep,
};

mod envelope;
mod length;
mod pulse_channel;
mod sweep;

#[derive(Clone, Default)]
pub struct Apu {
    is_on: bool,
    nr51: Nr51,
    nr50: Nr50,
    pub ch1: PulseChannel<Ch1Sweep>,
    pub ch2: PulseChannel<()>,
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
        flags.set(Nr52::CH4_ON, self.is_ch4_on());
        flags.set(Nr52::CH3_ON, self.is_ch3_on());
        flags.set(Nr52::CH2_ON, self.is_ch2_on());
        flags.set(Nr52::CH1_ON, self.is_ch1_on());
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

    fn is_ch1_on(&self) -> bool {
        self.ch1.is_on()
    }
    fn is_ch2_on(&self) -> bool {
        self.ch2.is_on()
    }
    fn is_ch3_on(&self) -> bool {
        false
    }
    fn is_ch4_on(&self) -> bool {
        false
    }
    pub fn execute(&mut self, div: u8) {
        if !self.is_on {
            return;
        }
        self.ch1.tick(div);
        self.ch2.tick(div);
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

#[derive(Default, Clone)]
pub struct NoiseChannel {
    length: Length<64>,
    volume_and_envelope: VolumeAndEnvelope,
    nr43: u8,
    is_enabled: bool,
}

impl NoiseChannel {
    pub fn write_nr41(&mut self, value: u8) {
        self.length.set_initial_timer_length(value);
    }
    pub fn read_nr41(&self) -> u8 {
        0xff
    }
    pub fn write_nr42(&mut self, value: u8) {
        self.volume_and_envelope.write_register(value);
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
    pub fn write_nr44(&mut self, value: u8) {
        self.length.is_enable = value & 0x40 != 0;
        if value & 0x80 != 0 {
            self.trigger();
        }
    }
    pub fn read_nr44(&self) -> u8 {
        ((self.length.is_enable as u8) << 6) | 0b10111111
    }

    fn trigger(&mut self) {
        self.is_enabled = true;
        self.length.trigger();
        self.volume_and_envelope.trigger();
    }

    fn is_on(&self) -> bool {
        self.volume_and_envelope.is_dac_on() && self.is_enabled && !self.length.is_expired()
    }

    fn tick(&mut self, div: u8) {
        if !self.is_on() {
            return;
        }
        self.length.tick(div);
        self.volume_and_envelope.tick(div);
    }
    fn get_divider(&self) -> u8 {
        self.nr43 & 0x7
    }
    fn get_shift(&self) -> u8 {
        (self.nr43 >> 4) & 0xf
    }
    fn get_tick_frequency(&self) -> f32 {
        // https://gbdev.io/pandocs/Audio_Registers.html#ff22--nr43-channel-4-frequency--randomness
        // Citation: Note that divider = 0 is treated as divider = 0.5 instead.
        let divider = self.get_divider();
        let divider: f32 = if divider == 0 { 0.5 } else { divider as f32 };
        262144.0 / (divider * 2.0f32.powi(self.get_shift().into()))
    }
    fn is_short_mode(&self) -> bool {
        self.nr43 & 0x8 != 0
    }

    fn sample(&self, sample: f32, noise: &[u8], short_noise: &[u8]) -> f32 {
        if !self.is_on() {
            return 0.;
        }

        let freq = self.get_tick_frequency();
        // if freq is equal to A Hz then it means the lfsr has emitted A different values in 1 second.
        // The noise is cyclic so we can use modulo if the index is greater than the provided noise values.
        let index = (sample * freq) as usize;

        (if self.is_short_mode() {
            short_noise[index % short_noise.len()] as f32
        } else {
            noise[index % noise.len()] as f32
        }) * self.volume_and_envelope.get_volume()
    }
}
