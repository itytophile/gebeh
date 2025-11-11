use crate::state::{State, WriteOnlyState, execute, fetch_opcode, get_instructions};
mod state;

fn main() {
    let mut state = State::default();
    let mut machine = OpCodeFetcher.compose(PipelineExecutor);

    loop {
        machine.execute(&state)(WriteOnlyState::new(&mut state));
    }
    // let rom = std::fs::read("/home/ityt/Téléchargements/pocket/pocket.gb").unwrap();
}

trait StateMachine {
    /// must take one M-cycle
    fn execute(&mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'static;
    fn compose<T: StateMachine>(self, other: T) -> (Self, T)
    where
        Self: Sized,
    {
        (self, other)
    }
}

struct OpCodeFetcher;
struct PipelineExecutor;

impl StateMachine for OpCodeFetcher {
    fn execute(&mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'static {
        let should_load_next_opcode = state.instruction_register.1.is_empty();
        // Every write here
        move |mut state| {
            if should_load_next_opcode {
                state.set_instruction_register(get_instructions(fetch_opcode()));
            }
        }
    }
}

impl StateMachine for PipelineExecutor {
    fn execute(&mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'static {
        let inst = state.instruction_register.0;
        move |mut state| {
            state.pipeline_pop_front();
            execute(inst);
        }
    }
}

impl<T: StateMachine, U: StateMachine> StateMachine for (T, U) {
    fn execute(&mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'static {
        let first = self.0.execute(state);
        let second = self.1.execute(state);
        move |mut state| {
            first(state.reborrow());
            second(state);
        }
    }
}
