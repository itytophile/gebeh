use crate::{
    cpu::PipelineExecutor,
    ppu::Ppu,
    state::{State, WriteOnlyState},
};
mod cpu;
mod dma;
mod gpu;
mod hardware;
mod ic;
mod instructions;
mod ppu;
mod state;

fn main() {
    // let rom =
    //     std::fs::read("/home/ityt/Téléchargements/pocket/pocket.gb")
    //         .unwrap();

    let mut state = State::default();
    // the machine should not be affected by the composition order
    let mut machine = PipelineExecutor::default().compose(Ppu::default());

    loop {
        machine.execute(&state)(WriteOnlyState::new(&mut state));
    }
}

trait StateMachine {
    /// must take one M-cycle
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a;
    fn compose<T: StateMachine>(self, other: T) -> (Self, T)
    where
        Self: Sized,
    {
        (self, other)
    }
}

impl<T: StateMachine, U: StateMachine> StateMachine for (T, U) {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        let first = self.0.execute(state);
        let second = self.1.execute(state);
        move |mut state| {
            first(state.reborrow());
            second(state);
        }
    }
}
