use crate::state::{State, WriteOnlyState, execute, fetch_opcode, get_instructions};
mod state;

fn main() {
    let mut state = State::default();
    let mut machine = OpCodeFetcher
        .compose(PipelineFeeder)
        .compose(PipelineExecutor);

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
struct PipelineFeeder;
struct PipelineExecutor;

impl StateMachine for OpCodeFetcher {
    fn execute(&mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'static {
        // Every read must be executed here
        let pipeline_len = state.pipeline.len();
        let is_opcode_none = state.instruction_register.is_none();
        // Every write here
        move |mut state| {
            if pipeline_len <= 1 && is_opcode_none {
                state.set_instruction_register(Some(fetch_opcode()));
            }
        }
    }
}

impl StateMachine for PipelineFeeder {
    fn execute(&mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'static {
        let is_pipeline_empty = state.pipeline.is_empty();
        let last_read_opcode = state.instruction_register;

        move |mut state| {
            if is_pipeline_empty && let Some(opcode) = last_read_opcode {
                state.extend_pipeline(get_instructions(opcode).iter().copied());
                state.set_instruction_register(None);
            }
        }
    }
}

impl StateMachine for PipelineExecutor {
    fn execute(&mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'static {
        let inst = state.pipeline.front().copied();
        move |mut state| {
            if let Some(inst) = inst {
                state.pipeline_pop_front();
                execute(inst);
            }
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
