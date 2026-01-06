use crate::apu::{envelope::VolumeAndEnvelope, length::Length};

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

    pub fn is_on(&self) -> bool {
        self.volume_and_envelope.is_dac_on() && self.is_enabled && !self.length.is_expired()
    }

    pub fn tick(&mut self, div: u8) {
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

    pub fn sample(&self, sample: f32, noise: &[u8], short_noise: &[u8]) -> f32 {
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
