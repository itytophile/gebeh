use arrayvec::ArrayVec;

#[derive(Clone, Copy, Debug)]
pub enum Register8Bit {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Clone, Copy, Debug)]
pub enum Register16Bit {
    AF,
    BC,
    DE,
    SP,
    HL,
}

#[derive(Clone, Copy, Debug)]
pub enum Flag {
    Z,
    N,
    H,
    C,
}

#[derive(Clone, Copy, Debug)]
pub struct Condition {
    pub flag: Flag,
    pub not: bool,
}

#[derive(Clone, Copy, Default, Debug)]
pub enum NoReadInstruction {
    #[default]
    Nop,
    Store8Bit(Register8Bit),
    Store16Bit(Register16Bit),
    Xor(Register8Bit),
    // Load to memory HL from A, Decrement
    LoadToAddressHlFromADec,
    Bit(u8, Register8Bit),
    OffsetPc,
    LoadFromAccumulator(Option<Register8Bit>),
    Inc(Register8Bit),
    LoadToAddressFromRegister {
        address: Register16Bit,
        value: Register8Bit,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum ReadInstruction {
    ReadLsb,
    ReadMsb,
    RelativeJump(Option<Condition>),
}

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    NoRead(NoReadInstruction),
    Read(ReadInstruction),
}

impl Default for Instruction {
    fn default() -> Self {
        Self::NoRead(NoReadInstruction::Nop)
    }
}

#[derive(Debug)]
pub enum AfterReadInstruction {
    NoRead(NoReadInstruction),
    Read(u8, ReadInstruction),
}

// always start with nop when cpu boots
// https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#fetch-and-stuff
pub type Instructions = (Instruction, ArrayVec<Instruction, 4>);

pub fn vec<const N: usize>(insts: [Instruction; N]) -> ArrayVec<Instruction, 4> {
    ArrayVec::from_iter(insts)
}

pub fn get_instructions(opcode: u8, is_cb_mode: bool) -> Instructions {
    use Instruction::*;
    use NoReadInstruction::*;
    use ReadInstruction::*;

    if is_cb_mode {
        return get_instructions_cb_mode(opcode);
    }

    match opcode {
        0 => Default::default(),
        // instructions in arrayvec are reversed
        0x0c => (NoRead(Inc(Register8Bit::C)), Default::default()),
        0x0e => (Read(ReadLsb), vec([NoRead(Store8Bit(Register8Bit::C))])),
        // When there is a jump we have to put a Nop even if the condition will be true
        // or the next opcode will be fetched with the wrong pc
        0x20 => (
            Read(RelativeJump(Some(Condition {
                flag: Flag::Z,
                not: true,
            }))),
            vec([NoRead(Nop)]),
        ),
        0x21 => (
            Read(ReadLsb),
            vec([NoRead(Store16Bit(Register16Bit::HL)), Read(ReadMsb)]),
        ),
        0x31 => (
            Read(ReadLsb),
            vec([NoRead(Store16Bit(Register16Bit::SP)), Read(ReadMsb)]),
        ),
        0x32 => (NoRead(LoadToAddressHlFromADec), vec([NoRead(Nop)])),
        0x3e => (Read(ReadLsb), vec([NoRead(Store8Bit(Register8Bit::A))])),
        0x77 => (
            NoRead(LoadToAddressFromRegister {
                address: Register16Bit::HL,
                value: Register8Bit::A,
            }),
            vec([NoRead(Nop)]),
        ),
        0xaf => (NoRead(Xor(Register8Bit::A)), Default::default()),
        0xcb => (NoRead(Nop), Default::default()),
        0xe0 => (
            Read(ReadLsb),
            vec([NoRead(Nop), NoRead(LoadFromAccumulator(None))]),
        ),
        0xe2 => (
            NoRead(LoadFromAccumulator(Some(Register8Bit::C))),
            vec([NoRead(Nop)]),
        ),
        _ => panic!("Opcode not implemented: 0x{opcode:02x}"),
    }
}

fn get_instructions_cb_mode(opcode: u8) -> Instructions {
    use Instruction::*;
    use NoReadInstruction::*;

    match opcode {
        0x7c => (NoRead(Bit(7, Register8Bit::H)), Default::default()),
        _ => panic!("Opcode not implemented (cb mode): 0x{opcode:02x}"),
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
        println!("Writing at ${address:04x} with value 0x{value:02x}");
        self.0.memory[usize::from(address)] = value;
    }
}
