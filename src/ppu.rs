use crate::{
    StateMachine,
    state::{State, WriteOnlyState},
};

struct Ppu;

// 4 dots per Normal Speed M-cycle
// One frame: 70224 dots
// A frame consists of 154 scanlines
// 456 dots per scanline

impl StateMachine for Ppu {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        |a| {}
    }
}
