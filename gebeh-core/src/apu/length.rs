#[derive(Clone, Default, Debug)]
pub struct Length<const L: u16> {
    pub is_enable: bool,
    initial_timer_length: u8,
    current_timer_value: u16,
}

impl Length<64> {
    pub fn set_initial_timer_length(&mut self, value: u8) {
        self.initial_timer_length = value & 0x3f;
    }
}

impl Length<256> {
    pub fn set_initial_timer_length(&mut self, value: u8) {
        self.initial_timer_length = value;
    }
}

impl<const L: u16> Length<L> {
    pub fn is_expired(&self) -> bool {
        self.current_timer_value == L
    }

    pub fn trigger(&mut self, force_reset: bool) {
        if self.is_expired() || force_reset {
            self.current_timer_value = u16::from(self.initial_timer_length);
        }
    }

    pub fn tick(&mut self) {
        // https://gbdev.io/pandocs/Audio_details.html#div-apu
        if !self.is_expired() && self.is_enable {
            self.current_timer_value += 1;
            log::info!("{} / {}", self.current_timer_value, L);
            if self.is_expired() {
                log::info!("Expiré !")
            }
        }
    }
}
