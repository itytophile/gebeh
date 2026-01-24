pub const MASK_8_BITS: u8 = 0xff;
pub const MASK_6_BITS: u8 = 0x3f;

#[derive(Clone, Default, Debug)]
pub struct Length<const MASK: u8> {
    pub is_enable: bool,
    current_timer_value: u8,
    has_overflowed: bool,
}

impl<const MASK: u8> Length<MASK> {
    pub fn set_initial_timer_length(&mut self, value: u8) {
        // according to blargg "Length can be reloaded at any time"
        self.current_timer_value = value & MASK;
    }

    pub fn is_expired(&self) -> bool {
        self.has_overflowed
    }

    pub fn trigger(&mut self) {
        log::info!("trigger {self:?}");
        self.has_overflowed = false;
    }

    pub fn tick(&mut self, cycles: u64) {
        // https://gbdev.io/pandocs/Audio_details.html#div-apu
        if !self.is_expired() && self.is_enable {
            self.current_timer_value = self.current_timer_value.wrapping_add(1) & MASK;
            self.has_overflowed = self.current_timer_value == 0;
        }
    }
}
