pub const MASK_8_BITS: u8 = 0xff;
pub const MASK_6_BITS: u8 = 0x3f;

#[derive(Clone, Default, Debug)]
pub struct Length<const MASK: u8> {
    pub is_enabled: bool,
    current_timer_value: Option<u8>, // None = overflowed
}

impl<const MASK: u8> Length<MASK> {
    pub fn set_initial_timer_length(&mut self, value: u8) {
        // according to blargg "Length can be reloaded at any time"
        self.current_timer_value = Some(MASK - (value & MASK));
    }

    pub fn trigger(&mut self, extra_clock: bool) {
        log::info!("trigger with extra = {}, {:?}", extra_clock, self);
        if self.current_timer_value.is_none() {
            // according to blargg "Trigger that un-freezes enabled length should clock it"
            self.current_timer_value = Some(MASK - (extra_clock && self.is_enabled) as u8);
        }
    }

    // returns true if overflow
    #[must_use]
    pub fn tick(&mut self, cycles: u64, ch: &'static str) -> bool {
        let (Some(prout), true) = (self.current_timer_value, self.is_enabled) else {
            return false;
        };

        log::info!("{cycles}: {ch} tick! {}", prout);

        self.current_timer_value = prout.checked_sub(1);

        self.current_timer_value.is_none()
    }
}
