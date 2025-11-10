use std::collections::VecDeque;

#[derive(Clone, Copy)]
struct Instruction;
struct OpCode;

fn fetch_opcode() -> OpCode {
    OpCode
}
fn execute(inst: Instruction) {}
fn get_instructions(opcode: OpCode) -> &'static [Instruction] {
    todo!()
}

fn main() {
    let mut state = State::default();
    let mut machine = OpCodeFetcher
        .compose(PipelineFeeder)
        .compose(PipelineExecutor);

    loop {
        machine.execute(&mut state);
    }
    // let rom = std::fs::read("/home/ityt/Téléchargements/pocket/pocket.gb").unwrap();
}

#[derive(Default)]
struct State {
    last_read_opcode: Option<OpCode>,
    pipeline: VecDeque<Instruction>,
}

trait StateMachine {
    /// must take one M-cycle
    fn execute(&mut self, state: &mut State);
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
    fn execute(&mut self, state: &mut State) {
        if state.pipeline.len() <= 1 && state.last_read_opcode.is_none() {
            state.last_read_opcode = Some(fetch_opcode());
        }
    }
}

impl StateMachine for PipelineFeeder {
    fn execute(&mut self, state: &mut State) {
        if state.pipeline.is_empty()
            && let Some(opcode) = state.last_read_opcode.take()
        {
            state.pipeline.extend(get_instructions(opcode));
        }
    }
}

impl StateMachine for PipelineExecutor {
    fn execute(&mut self, state: &mut State) {
        if let Some(inst) = state.pipeline.pop_front() {
            execute(inst);
        }
    }
}

impl<T: StateMachine, U: StateMachine> StateMachine for (T, U) {
    fn execute(&mut self, state: &mut State) {
        self.0.execute(state);
        self.1.execute(state);
    }
}
