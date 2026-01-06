use crate::apu::length::Length;

#[derive(Default, Clone)]
pub struct WaveChannel {
    is_enabled: bool,
    is_dac_on: bool,
    length: Length<256>,
    output_level: u8, // 2 bits
    effective_output_level: u8,
    period: u16, // 11 bits
    ram: [u8; 16],
}

impl WaveChannel {
    pub fn get_nr30(&self) -> u8 {
        ((self.is_dac_on as u8) << 7) | 0b01111111
    }
    pub fn write_nr30(&mut self, value: u8) {
        self.is_dac_on = value & 0x80 != 0;
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
        ((self.length.is_enable as u8) << 6) | 0b10111111
    }
    pub fn write_nr34(&mut self, value: u8) {
        self.period = (u16::from(value & 0x07) << 8) | self.period & 0x00ff;
        self.length.is_enable = value & 0x40 != 0;
        if value & 0x80 != 0 {
            self.trigger();
        }
    }
    fn trigger(&mut self) {
        self.is_enabled = true;
        self.length.trigger();
        self.effective_output_level = self.output_level;
    }
    pub fn is_on(&self) -> bool {
        self.is_enabled && self.is_dac_on && !self.length.is_expired()
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
    pub fn sample(&self, sample: f32) -> f32 {
        // About output level https://gbdev.io/pandocs/Audio_Registers.html#ff1c--nr32-channel-3-output-level
        if !self.is_on() || self.effective_output_level == 0 {
            return 0.;
        }

        let index = (sample * self.get_tone_frequency()) % 1.;
        let index = (index * 32.) as usize;
        let two_samples = self.ram[index / 2];

        // https://gbdev.io/pandocs/Audio_Registers.html#ff30ff3f--wave-pattern-ram
        // Citation: As CH3 plays, it reads wave RAM left to right, upper nibble first
        let value = (if index.is_multiple_of(2) {
            two_samples >> 4
        } else {
            two_samples & 0x0f
        }) >> (self.effective_output_level - 1);

        value as f32 / 0x0f as f32
    }

    // https://gbdev.io/pandocs/Audio_Registers.html#ff1d--nr33-channel-3-period-low-write-only
    fn get_tone_frequency(&self) -> f32 {
        65536. / (2048. - self.period as f32)
    }

    pub fn tick(&mut self, div: u8) {
        if !self.is_on() {
            return;
        }
        self.length.tick(div);
    }
}
