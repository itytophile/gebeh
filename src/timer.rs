use crate::{
    StateMachine,
    ic::Ints,
    state::{State, WriteOnlyState},
};

#[derive(Default)]
pub struct Timer(u8);

impl StateMachine for Timer {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        self.0 = self.0.wrapping_add(1);
        let increment_frequency: u16 = match state.timer_control & 0b11 {
            0 => 256,
            1 => 4,
            0b10 => 16,
            0b11 => 64,
            _ => unreachable!(),
        };
        let mut timer_counter = state.timer_counter;
        let mut overflow = false;
        if u16::from(self.0) % increment_frequency == 0 {
            timer_counter = if let Some(value) = timer_counter.checked_add(1) {
                value
            } else {
                overflow = true;
                state.timer_modulo
            };
        }

        move |mut state| {
            state.set_timer_counter(timer_counter);
            if overflow {
                state.get_if_mut().insert(Ints::TIMER);
            }
        }
    }
}
