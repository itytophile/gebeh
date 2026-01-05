#[derive(Clone, Default)]
pub struct Length<const L: u16> {
    pub is_enable: bool,
    initial_timer_length: u8,
    current_timer_value: u16,
    falling_edge: bool,
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

    pub fn trigger(&mut self) {
        if self.is_expired() {
            self.current_timer_value = u16::from(self.initial_timer_length);
        }
    }

    pub fn tick(&mut self, div: u8) {
        if self.is_expired() || !self.is_enable {
            return;
        }

        // 256 Hz
        let has_ticked = div & (1 << 5) != 0;

        if self.falling_edge == has_ticked {
            return;
        }

        self.falling_edge = has_ticked;

        if !self.falling_edge {
            self.current_timer_value += 1;
        }
    }
}
