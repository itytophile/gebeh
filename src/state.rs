use arrayvec::ArrayVec;

#[derive(Clone, Copy, Debug)]
pub enum Register8Bit {
    A,
}

#[derive(Clone, Copy, Debug)]
pub enum Register16Bit {
    SP,
    HL,
}

#[derive(Clone, Copy, Default, Debug)]
pub enum NoReadInstruction {
    #[default]
    Nop,
    Store16Bit(Register16Bit),
    Xor(Register8Bit),
    // Load to memory HL from A, Decrement
    LoadToMhlFromADec,
}

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    NoRead(NoReadInstruction),
    ReadLsb,
    ReadMsb,
}

impl Default for Instruction {
    fn default() -> Self {
        Self::NoRead(NoReadInstruction::Nop)
    }
}

#[derive(Debug)]
pub enum AfterReadInstruction {
    NoRead(NoReadInstruction),
    ReadLsb(u8),
    ReadMsb(u8),
}

// always start with nop when cpu boots
// https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#fetch-and-stuff
pub type Instructions = (Instruction, ArrayVec<Instruction, 4>);

pub fn get_instructions(opcode: u8) -> Instructions {
    use Instruction::*;
    use NoReadInstruction::*;
    match opcode {
        0 => Default::default(),
        // instructions in arrayvec is reversed
        0x21 => (
            ReadLsb,
            ArrayVec::from_iter([NoRead(Store16Bit(Register16Bit::HL)), ReadMsb]),
        ),
        0x31 => (
            ReadLsb,
            ArrayVec::from_iter([NoRead(Store16Bit(Register16Bit::SP)), ReadMsb]),
        ),
        0x32 => (NoRead(LoadToMhlFromADec), Default::default()),
        0xaf => (NoRead(Xor(Register8Bit::A)), Default::default()),
        _ => panic!("Opcode not implemented: 0x{opcode:02x}"),
    }
}

// une instruction prend plusieurs m-cycles
// l'opcode détermine quel instruction exécuter
// À l'exécution du dernier M-cycle d'une instruction, le prochain opcode est chargé en parallèle

pub struct State {
    pub instruction_register: Instructions,
    pub pc: u16,
    pub memory: [u8; 0x10000],
}

impl Default for State {
    fn default() -> Self {
        Self {
            instruction_register: Default::default(),
            pc: Default::default(),
            memory: [0; 0x10000],
        }
    }
}

pub struct WriteOnlyState<'a>(&'a mut State);

impl<'a> WriteOnlyState<'a> {
    pub fn new(state: &'a mut State) -> Self {
        Self(state)
    }
    pub fn reborrow<'c>(&'c mut self) -> WriteOnlyState<'c>
    where
        'a: 'c,
    {
        WriteOnlyState(&mut *self.0)
    }
    pub fn set_instruction_register(&mut self, instructions: Instructions) {
        self.0.instruction_register = instructions;
    }

    pub fn pipeline_pop_front(&mut self) {
        // does nothing if there is only one instruction inside the pipeline
        // if there is only one instruction then the OpcodeFetcher will override the whole pipeline
        if let Some(next_inst) = self.0.instruction_register.1.pop() {
            self.0.instruction_register.0 = next_inst;
        }
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.0.pc = pc;
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.0.memory[usize::from(address)] = value;
    }
}
