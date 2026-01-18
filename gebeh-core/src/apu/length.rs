use crate::apu::FallingEdge;

#[derive(Clone, Default)]
pub struct Length<const L: u16> {
    pub is_enable: bool,
    initial_timer_length: u8,
    current_timer_value: u16,
    falling_edge: FallingEdge,
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

    pub fn tick(&mut self, div_apu: u8) {
        // https://gbdev.io/pandocs/Audio_details.html#div-apu
        if !self.is_expired()
            && self.is_enable
            && self.falling_edge.update(!div_apu.is_multiple_of(2))
        {
            self.current_timer_value += 1;
        }
    }
}
