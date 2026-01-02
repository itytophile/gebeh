#[derive(Clone, Default)]
struct Ch1Sweep {
    nr10: u8,
}

trait Sweep {
    fn trigger(&mut self);
    fn tick(&mut self) -> bool;
}

impl Sweep for Ch1Sweep {
    fn trigger(&mut self) {
        todo!()
    }
    // Returns channel on/off
    fn tick(&mut self) -> bool {
        todo!()
    }
}

impl Sweep for () {
    fn trigger(&mut self) {}

    fn tick(&mut self) -> bool {
        true
    }
}

#[derive(Clone, Default)]
struct PulseChannel<S: Sweep> {
    length_timer_and_duty_cycle: u8,
    volume_and_envelope: u8,
    period_low: u8,
    period_high_and_control: u8,
    is_enabled: bool,
    sweep: S,
}

impl<S: Sweep> PulseChannel<S> {
    fn set_period_high_and_control(&mut self, value: u8) {
        let is_triggered = value & 0x80 != 0;
        self.period_high_and_control = value;
        if is_triggered {
            self.trigger();
        }
    }

    fn trigger(&mut self) {
        self.is_enabled = true;
        if self.is_length_timer_expired() {
            self.reset_length_timer();
        }
        self.reload_period_divider();
        self.reset_envelope_timer();
        self.reload_volume();
        self.sweep.trigger();
    }

    fn is_on(&self) -> bool {
        self.is_dac_on() && self.is_enabled
    }

    fn is_length_timer_expired(&self) -> bool {
        todo!()
    }

    fn is_length_timer_enabled(&self) -> bool {
        todo!()
    }

    fn is_dac_on(&self) -> bool {
        // https://gbdev.io/pandocs/Audio_details.html#dacs
        self.volume_and_envelope & 0xf8 != 0
    }

    fn reset_length_timer(&self) {
        todo!()
    }

    fn reload_period_divider(&self) {
        todo!()
    }

    fn reset_envelope_timer(&self) {
        todo!()
    }

    fn reload_volume(&self) {
        todo!()
    }

    fn tick(&mut self) {
        if !self.is_on() {
            return;
        }
        self.is_enabled = self.sweep.tick();
    }
}

#[derive(Clone, Default)]
pub struct Apu {
    is_on: bool,
    nr51: Nr51,
    nr50: Nr50,
    ch1: PulseChannel<Ch1Sweep>,
    ch2: PulseChannel<()>,
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

    fn is_ch1_on(&self) -> bool {
        todo!()
    }
    fn is_ch2_on(&self) -> bool {
        todo!()
    }
    fn is_ch3_on(&self) -> bool {
        todo!()
    }
    fn is_ch4_on(&self) -> bool {
        todo!()
    }
}
