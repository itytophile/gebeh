use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub struct Instruction;
#[derive(Clone, Copy)]
pub enum OpCode {
    Nop,
}

pub fn fetch_opcode() -> OpCode {
    OpCode::Nop
}
pub fn execute(inst: Instruction) {}
pub fn get_instructions(opcode: OpCode) -> &'static [Instruction] {
    todo!()
}

pub struct State {
    pub instruction_register: Option<OpCode>,
    pub pipeline: VecDeque<Instruction>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            // https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#fetch-and-stuff
            instruction_register: Some(OpCode::Nop),
            pipeline: Default::default(),
        }
    }
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
    pub fn set_instruction_register(&mut self, value: Option<OpCode>) {
        self.0.instruction_register = value;
    }
    pub fn extend_pipeline<T: IntoIterator<Item = Instruction>>(&mut self, iter: T) {
        self.0.pipeline.extend(iter);
    }
    pub fn pipeline_pop_front(&mut self) {
        self.0.pipeline.pop_front();
    }
}
