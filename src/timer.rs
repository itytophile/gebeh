use crate::{
    StateMachine,
    ic::Ints,
    state::{State, WriteOnlyState},
};

#[derive(Default)]
pub struct Timer(u8);

impl StateMachine for Timer {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        self.0 = self.0.wrapping_add(1);
        let div = state.div;
        let increment_frequency: u16 = match state.timer_control & 0b11 {
            0 => 256,
            1 => 4,
            0b10 => 16,
            0b11 => 64,
            _ => unreachable!(),
        };
        let mut timer_counter = state.timer_counter;
        let mut overflow = false;
        if state.timer_control & 0b100 == 0b100 && u16::from(self.0) % increment_frequency == 0 {
            timer_counter = if let Some(value) = timer_counter.checked_add(1) {
                value
            } else {
                overflow = true;
                state.timer_modulo
            };
        }

        Some(move |mut state: WriteOnlyState| {
            state.set_timer_counter(timer_counter);
            state.set_div(div.wrapping_add((self.0 == 0) as u8));
            if overflow {
                state.insert_if(Ints::TIMER);
            }
        })
    }
}
