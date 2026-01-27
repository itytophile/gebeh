pub const MASK_8_BITS: u8 = 0xff;
pub const MASK_6_BITS: u8 = 0x3f;

#[derive(Clone, Default, Debug)]
pub struct Length<const MASK: u8> {
    is_enabled: bool,
    current_timer_value: Option<u8>, // None = overflowed
}

pub fn is_length_extra_clock(div_apu: u8) -> bool {
    div_apu % 2 == 1
}

impl<const MASK: u8> Length<MASK> {
    #[must_use]
    pub fn reset(&self) -> Self {
        Self {
            current_timer_value: self.current_timer_value,
            ..Default::default()
        }
    }
    // returns true if overflow
    #[must_use]
    pub fn set_is_enabled(&mut self, is_enabled: bool, div_apu: u8) -> bool {
        let previous_is_length_enabled = self.is_enabled;
        self.is_enabled = is_enabled;

        // according to blargg "Enabling in first half of length period should clock length"
        if !previous_is_length_enabled && self.is_enabled && is_length_extra_clock(div_apu) {
            return self.tick();
        }

        false
    }
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }
    pub fn set_initial_timer_length(&mut self, value: u8) {
        // according to blargg "Length can be reloaded at any time"
        self.current_timer_value = Some(MASK - (value & MASK));
    }

    pub fn trigger(&mut self, div_apu: u8) {
        // according to GameRoy there is an extra clock here
        let extra_clock = is_length_extra_clock(div_apu);
        if self.current_timer_value.is_none() {
            // according to blargg "Trigger that un-freezes enabled length should clock it"
            self.current_timer_value = Some(MASK - (extra_clock && self.is_enabled) as u8);
        }
    }

    // returns true if overflow
    #[must_use]
    pub fn tick(&mut self) -> bool {
        let (Some(value), true) = (self.current_timer_value, self.is_enabled) else {
            return false;
        };

        self.current_timer_value = value.checked_sub(1);

        self.current_timer_value.is_none()
    }
}
