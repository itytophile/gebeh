use crate::{
    StateMachine,
    state::{State, WriteOnlyState},
};

pub struct Ppu2;

impl StateMachine for Ppu2 {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        Some(|_: WriteOnlyState| {})
    }
}
