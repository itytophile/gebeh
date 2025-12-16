use crate::{
    StateMachine,
    ic::Ints,
    state::{State, WriteOnlyState},
};

// There is a system counter which is 14 bits wide
// The div register is the height most significant bits of this system counter
// the system counter is incremented every m-cycle
// i.e the div counter is incremented every m-cycle * 2^6
// m-cycle frequency = 4.194304 MHz / 4
// so div frequency = 4.194304 MHz / 4 / 2^6 = 16384 Hz as pandocs says
// https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff04--div-divider-register

#[derive(Clone)]
pub struct Timer;

impl StateMachine for Timer {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        let reset = state.reset_system_clock;

        let increment_frequency: u16 = match state.timer_control & 0b11 {
            0 => 256,
            1 => 4,
            0b10 => 16,
            0b11 => 64,
            _ => unreachable!(),
        };
        let mut timer_counter = state.timer_counter;
        let mut overflow = false;
        let system_counter = if reset {
            0
        } else {
            state.system_counter.wrapping_add(1)
        };
        if state.timer_control & 0b100 == 0b100
            && system_counter.is_multiple_of(increment_frequency)
        {
            timer_counter = if let Some(value) = timer_counter.checked_add(1) {
                value
            } else {
                overflow = true;
                state.timer_modulo
            };
        }

        Some(move |mut state: WriteOnlyState| {
            state.set_timer_counter(timer_counter);
            if reset {
                state.reset_system_counter();
            } else {
                state.increment_system_counter();
            }
            state.set_reset_system_clock(false);
            if overflow {
                state.insert_if(Ints::TIMER);
            }
        })
    }
}
