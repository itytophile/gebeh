use crate::apu::{envelope::VolumeAndEnvelope, length::Length};

mod envelope;
mod length;

type Wave = [f32; 8];

// https://gbdev.io/pandocs/Audio_Registers.html#ff11--nr11-channel-1-length-timer--duty-cycle
const WAVE_00: Wave = [1., 1., 1., 1., 1., 1., 1., -1.];
const WAVE_01: Wave = [-1., 1., 1., 1., 1., 1., 1., -1.];
const WAVE_10: Wave = [-1., 1., 1., 1., 1., -1., -1., -1.];
const WAVE_11: Wave = [1., -1., -1., -1., -1., -1., -1., 1.];

#[derive(Clone, Default)]
pub struct Ch1Sweep {
    nr10: u8,
    pace_count: u8,
    falling_edge: bool,
    period_value: u16,
    // https://gbdev.io/pandocs/Audio_Registers.html#ff10--nr10-channel-1-sweep
    // Citation: Note that the value written to this field is not re-read by the hardware
    // until a sweep iteration completes, or the channel is (re)triggered.
    pace: u8,
    // https://gbdev.io/pandocs/Audio_details.html#pulse-channel-with-sweep-ch1
    // The “enabled flag” is set if either the sweep pace or individual step are non-zero, cleared otherwise.
    is_enabled: bool,
}

impl Ch1Sweep {
    fn is_decreasing(&self) -> bool {
        self.nr10 & 0x08 != 0
    }
    // 3 bits
    fn individual_step(&self) -> u8 {
        self.nr10 & 0x07
    }

    // None -> overflow
    fn compute_next_value_and_check_overflow(&self) -> Option<u16> {
        if self.is_decreasing() {
            return Some(self.period_value - (self.period_value >> self.individual_step()));
        }

        let new_period = self.period_value + (self.period_value >> self.individual_step());

        if new_period > 0x7ff {
            return None;
        }

        Some(new_period)
    }
}

pub trait Sweep {
    // returns new period value
    #[must_use]
    fn trigger(&mut self, period: u16) -> Option<u16>;
    // is channel still enable, new period value
    #[must_use]
    fn tick(&mut self, div: u8) -> (bool, Option<u16>);
    #[must_use]
    fn get_period_value(&self) -> Option<u16>;
}

impl Sweep for Ch1Sweep {
    fn trigger(&mut self, period: u16) -> Option<u16> {
        self.period_value = period;
        self.pace_count = 0;
        self.pace = (self.nr10 >> 4) & 0x07;
        self.is_enabled = self.pace != 0 || self.individual_step() != 0;
        // https://gbdev.io/pandocs/Audio_details.html#pulse-channel-with-sweep-ch1
        // Citation: If the individual step is non-zero, frequency calculation and overflow check are performed immediately.
        if self.individual_step() != 0
            && let Some(new_period) = self.compute_next_value_and_check_overflow()
        {
            self.period_value = new_period;
            return Some(new_period);
        }

        None
    }

    // Returns channel on/off
    fn tick(&mut self, div: u8) -> (bool, Option<u16>) {
        if !self.is_enabled {
            return (true, None);
        }
        // https://gbdev.io/pandocs/Audio_Registers.html#ff10--nr10-channel-1-sweep
        // Citation: In addition mode, if the period value would overflow (i.e. Lt+1 is
        // strictly more than $7FF), the channel is turned off instead. This occurs even
        // if sweep iterations are disabled by the pace being 0.
        let Some(new_period_value) = self.compute_next_value_and_check_overflow() else {
            return (false, None);
        };

        if self.pace == 0 {
            return (true, None);
        }
        // 128 Hz
        let has_ticked = div & (1 << 4) != 0;

        if self.falling_edge == has_ticked {
            return (true, None);
        }

        self.falling_edge = has_ticked;

        self.pace_count += 1;

        if self.pace_count != self.pace {
            return (true, None);
        }

        self.pace_count = 0;
        self.period_value = new_period_value;

        // https://gbdev.io/pandocs/Audio_Registers.html#ff10--nr10-channel-1-sweep
        // Citation: Note that the value written to this field is not re-read by the hardware until a sweep iteration completes
        if new_period_value == 0 {
            self.pace = (self.nr10 >> 4) & 0x07;
        }

        // https://gbdev.io/pandocs/Audio_details.html#pulse-channel-with-sweep-ch1
        // Citation: then frequency calculation and overflow check are run again immediately
        // using this new value, but this second new frequency is not written back
        if self.compute_next_value_and_check_overflow().is_none() {
            return (false, Some(new_period_value));
        }

        (true, Some(new_period_value))
    }

    fn get_period_value(&self) -> Option<u16> {
        if self.pace == 0 && self.individual_step() == 0 {
            None
        } else {
            Some(self.period_value)
        }
    }
}

impl Sweep for () {
    fn trigger(&mut self, _: u16) -> Option<u16> {
        None
    }

    fn tick(&mut self, _: u8) -> (bool, Option<u16>) {
        (true, None)
    }

    fn get_period_value(&self) -> Option<u16> {
        None
    }
}

#[derive(Clone, Default)]
pub struct PulseChannel<S: Sweep> {
    length: Length<64>,
    duty_cycle: u8,
    volume_and_envelope: VolumeAndEnvelope,
    period_low: u8,
    period_high: u8,
    is_enabled: bool,
    sweep: S,
}

impl<S: Sweep> PulseChannel<S> {
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
    pub fn write_nrx4(&mut self, value: u8) {
        self.length.is_enable = value & 0x40 != 0;
        self.period_high = value & 0x07;
        if value & 0x80 != 0 {
            self.trigger();
        }
    }

    fn trigger(&mut self) {
        self.is_enabled = true;
        self.length.trigger();
        self.volume_and_envelope.trigger();
        if let Some(new_period) = self.sweep.trigger(self.get_period_value()) {
            self.set_period_value(new_period);
        }
        log::warn!("new tone frequency: {} Hz", self.get_tone_frequency());
    }

    fn is_on(&self) -> bool {
        self.volume_and_envelope.is_dac_on() && self.is_enabled && !self.length.is_expired()
    }

    // must be called at 1048576 Hz (once per four dots)
    fn tick(&mut self, div: u8) {
        if !self.is_on() {
            return;
        }
        // don't use && directly because it is lazy
        // https://doc.rust-lang.org/reference/expressions/operator-expr.html#r-expr.bool-logic.conditional-evaluation
        let (is_enabled_from_sweep, new_period) = self.sweep.tick(div);
        if let Some(period) = new_period {
            self.set_period_value(period);
        }
        self.is_enabled = is_enabled_from_sweep;
        self.length.tick(div);
        self.volume_and_envelope.tick(div);
    }

    // 11 bits
    fn get_period_value(&self) -> u16 {
        u16::from_be_bytes([self.period_high & 0x07, self.period_low])
    }

    fn set_period_value(&mut self, value: u16) {
        self.period_low = value as u8;
        self.period_high = self.period_high & 0b11000000 | ((value >> 4) as u8) & 0x07;
    }

    fn sample(&self, sample: f32) -> f32 {
        if !self.is_on() {
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
        wave[index] * self.volume_and_envelope.get_volume()
    }
    // https://gbdev.io/pandocs/Audio_Registers.html#ff13--nr13-channel-1-period-low-write-only
    fn get_tone_frequency(&self) -> f32 {
        131072.0
            / (2048.0
                - self
                    .sweep
                    .get_period_value()
                    .unwrap_or(self.get_period_value()) as f32)
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

    /// sample in [0;1[
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

    /// sample in [0;1[
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
        // So we must index the noise values in the interval [0; A[
        // The noise is cyclic so we can use modulo if the index is greater than the provided noise values.
        // (Remainder: freq is in [0;1[)
        let index = (sample * freq) as usize;

        (if self.is_short_mode() {
            short_noise[index % short_noise.len()] as f32
        } else {
            noise[index % noise.len()] as f32
        }) * self.volume_and_envelope.get_volume()
    }
}
