use crate::state::{State, WriteOnlyState, execute, get_instructions};
mod state;

fn main() {
    let rom =
        std::fs::read("/home/ityt/Téléchargements/pocket/pocket.gb")
            .unwrap();

    let mut state = State::default();
    let mut machine = OpCodeFetcher::new(&rom).compose(PipelineExecutor);

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

struct OpCodeFetcher<'a> {
    pc: u16,
    rom: &'a [u8],
}

impl<'a> OpCodeFetcher<'a> {
    fn new(rom: &'a [u8]) -> Self {
        Self { pc: 0, rom }
    }
}

struct PipelineExecutor;

impl StateMachine for OpCodeFetcher<'_> {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        // we load the next opcode if there is only one instruction left in the pipeline
        let should_load_next_opcode = state.instruction_register.1.is_empty();
        // Every write here
        move |mut state| {
            if should_load_next_opcode {
                state.set_instruction_register(get_instructions(self.rom[usize::from(self.pc)]));
                self.pc += 1;
            }
        }
    }
}

impl StateMachine for PipelineExecutor {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        let inst = state.instruction_register.0;
        move |mut state| {
            state.pipeline_pop_front();
            execute(inst);
        }
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
