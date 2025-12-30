use crate::{ic::Ints, state::State};

// There is a system counter which is 14 bits wide
// The div register is the height most significant bits of this system counter
// the system counter is incremented every m-cycle
// i.e the div counter is incremented every m-cycle * 2^6
// m-cycle frequency = 4.194304 MHz / 4
// so div frequency = 4.194304 MHz / 4 / 2^6 = 16384 Hz as pandocs says
// https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff04--div-divider-register

#[derive(Clone)]
pub struct Timer;

impl Timer {
    pub fn execute(&mut self, state: &mut State, _: u64) {
        let increment_frequency: u16 = match state.timer_control & 0b11 {
            0 => 256,
            1 => 4,
            0b10 => 16,
            0b11 => 64,
            _ => unreachable!(),
        };

        state.system_counter = state.system_counter.wrapping_add(1);

        if state.timer_control & 0b100 != 0b100
            || !state.system_counter.is_multiple_of(increment_frequency)
        {
            return;
        }

        state.timer_counter = if let Some(value) = state.timer_counter.checked_add(1) {
            value
        } else {
            state.interrupt_flag.insert(Ints::TIMER);
            state.timer_modulo
        };
    }
}
