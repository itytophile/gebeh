#[derive(Clone, Default, Debug)]
pub struct Length<const L: u16> {
    pub is_enable: bool,
    initial_timer_length: u8,
    current_timer_value: u16,
}

impl Length<64> {
    pub fn set_initial_timer_length(&mut self, value: u8) {
        self.initial_timer_length = value & 0x3f;
        // according to blargg "Length can be reloaded at any time"
        self.current_timer_value = u16::from(self.initial_timer_length);
    }
}

impl Length<256> {
    pub fn set_initial_timer_length(&mut self, value: u8) {
        self.initial_timer_length = value;
        // according to blargg "Length can be reloaded at any time"
        self.current_timer_value = u16::from(self.initial_timer_length);
    }
}

impl<const L: u16> Length<L> {
    pub fn is_expired(&self) -> bool {
        self.current_timer_value == L
    }

    pub fn trigger(&mut self) {
        log::info!("trigger {self:?}");
        if self.is_expired() {
            log::info!("Setting to {}", self.initial_timer_length);
            // according to blargg, it is not reset to the initial_timer_length but to 0
            self.current_timer_value = 0;
        }
    }

    pub fn tick(&mut self, cycles: u64) {
        // https://gbdev.io/pandocs/Audio_details.html#div-apu
        if !self.is_expired() && self.is_enable {
            self.current_timer_value += 1;
            log::info!("{cycles}: {} / {}", self.current_timer_value, L);
            if self.is_expired() {
                log::info!("Expiré !")
            }
        }
    }
}
