pub const MASK_8_BITS: u8 = 0xff;
pub const MASK_6_BITS: u8 = 0x3f;

#[derive(Clone, Default, Debug)]
pub struct Length<const MASK: u8> {
    pub is_enable: bool,
    current_timer_value: u8,
}

impl<const MASK: u8> Length<MASK> {
    pub fn set_initial_timer_length(&mut self, value: u8) {
        // according to blargg "Length can be reloaded at any time"
        self.current_timer_value = MASK - (value & MASK);
    }

    pub fn trigger(&mut self) {
        log::info!("trigger {:?}", self);
        if self.current_timer_value == 0 {
            self.current_timer_value = MASK;
        }
    }

    // returns true if overflow
    #[must_use]
    pub fn tick(&mut self, cycles: u64, ch: &'static str) -> bool {
        // https://gbdev.io/pandocs/Audio_details.html#div-apu
        if self.is_enable {
            log::info!("{cycles}: {ch} tick! {}", self.current_timer_value);
            if let Some(lol) = self.current_timer_value.checked_sub(1) {
                self.current_timer_value = lol;
            } else {
                log::info!("{ch} Expired!");
                return true;
            }
        }

        false
    }
}
