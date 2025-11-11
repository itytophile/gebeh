use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub struct Instruction;
#[derive(Clone, Copy)]
pub struct OpCode;

pub fn fetch_opcode() -> OpCode {
    OpCode
}
pub fn execute(inst: Instruction) {}
pub fn get_instructions(opcode: OpCode) -> &'static [Instruction] {
    todo!()
}

#[derive(Default)]
pub struct State {
    pub last_read_opcode: Option<OpCode>,
    pub pipeline: VecDeque<Instruction>,
}

pub struct WriteOnlyState<'a>(&'a mut State);

impl<'a> WriteOnlyState<'a> {
    pub fn new(state: &'a mut State) -> Self {
        Self(state)
    }
    pub fn reborrow<'b>(&'b mut self) -> WriteOnlyState<'b>
    where
        'a: 'b,
    {
        WriteOnlyState(&mut *self.0)
    }
    pub fn set_last_read_opcode(&mut self, value: Option<OpCode>) {
        self.0.last_read_opcode = value;
    }
    pub fn extend_pipeline<T: IntoIterator<Item = Instruction>>(&mut self, iter: T) {
        self.0.pipeline.extend(iter);
    }
    pub fn pipeline_pop_front(&mut self) {
        self.0.pipeline.pop_front();
    }
}
