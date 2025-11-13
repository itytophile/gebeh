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
    F,
}

#[derive(Clone, Copy, Debug)]
pub enum Register16Bit {
    AF,
    BC,
    DE,
    SP,
    HL,
}

impl Register16Bit {
    fn get_msb(self) -> Register8Bit {
        match self {
            Register16Bit::AF => Register8Bit::A,
            Register16Bit::BC => Register8Bit::B,
            Register16Bit::DE => Register8Bit::D,
            Register16Bit::SP => unreachable!(),
            Register16Bit::HL => Register8Bit::H,
        }
    }

    fn get_lsb(self) -> Register8Bit {
        match self {
            Register16Bit::AF => Register8Bit::F,
            Register16Bit::BC => Register8Bit::C,
            Register16Bit::DE => Register8Bit::E,
            Register16Bit::SP => unreachable!(),
            Register16Bit::HL => Register8Bit::L,
        }
    }
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
    DecStackPointer,
    WriteMsbOfRegisterWhereSpPointsAndDecSp(Option<Register16Bit>),
    WriteLsbPcWhereSpPointsAndLoadCacheToPc,
    Load {
        to: Register8Bit,
        from: Register8Bit,
    },
    Rl(Register8Bit),
    Rla
}

#[derive(Clone, Copy, Debug)]
pub enum ReadInstruction {
    ReadIntoLsb,
    ReadIntoMsb,
    RelativeJump(Option<Condition>),
}

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    NoRead(NoReadInstruction),
    Read(Option<Register16Bit>, ReadInstruction),
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
pub type Instructions = (Instruction, ArrayVec<Instruction, 5>);

pub fn vec<const N: usize>(insts: [Instruction; N]) -> ArrayVec<Instruction, 5> {
    ArrayVec::from_iter(insts)
}

mod opcodes {
    use super::Instruction::*;
    use super::Instructions;
    use super::NoReadInstruction::*;
    use super::ReadInstruction::*;
    use super::Register8Bit;
    use super::Register16Bit;
    use super::vec;

    pub fn ld_r_n(register: Register8Bit) -> Instructions {
        (Read(None, ReadIntoLsb), vec([NoRead(Store8Bit(register))]))
    }

    pub fn ld_rr_n(register: Register16Bit) -> Instructions {
        (
            Read(None, ReadIntoLsb),
            vec([NoRead(Store16Bit(register)), Read(None, ReadIntoMsb)]),
        )
    }

    pub fn push_rr(register: Register16Bit) -> Instructions {
        (
            NoRead(DecStackPointer),
            vec([
                NoRead(Nop),
                NoRead(LoadToAddressFromRegister {
                    address: Register16Bit::SP,
                    value: register.get_lsb(),
                }),
                NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(Some(register))),
            ]),
        )
    }

    pub fn bit_b_r(bit: u8, register: Register8Bit) -> Instructions {
        (NoRead(Bit(bit, register)), Default::default())
    }

    pub fn rl_r(register: Register8Bit) -> Instructions {
        (NoRead(Rl(register)), Default::default())
    }
}

use opcodes::*;

pub fn get_instructions(opcode: u8, is_cb_mode: bool) -> Instructions {
    use Instruction::*;
    use NoReadInstruction::*;
    use ReadInstruction::*;

    if is_cb_mode {
        return get_instructions_cb_mode(opcode);
    }

    // instructions in arrayvec are reversed
    match opcode {
        0 => Default::default(),
        0x01 => ld_rr_n(Register16Bit::BC),
        0x0c => (NoRead(Inc(Register8Bit::C)), Default::default()),
        0x0e => ld_r_n(Register8Bit::C),
        0x06 => ld_r_n(Register8Bit::B),
        0x11 => ld_rr_n(Register16Bit::DE),
        0x17 => (NoRead(Rla), Default::default()),
        0x1e => ld_r_n(Register8Bit::E),
        0x16 => ld_r_n(Register8Bit::D),
        0x1a => (
            Read(Some(Register16Bit::DE), ReadIntoLsb),
            vec([NoRead(Store8Bit(Register8Bit::A))]),
        ),
        // When there is a jump we have to put a Nop even if the condition will be true
        // or the next opcode will be fetched with the wrong pc
        0x20 => (
            Read(
                None,
                RelativeJump(Some(Condition {
                    flag: Flag::Z,
                    not: true,
                })),
            ),
            vec([NoRead(Nop)]),
        ),
        0x21 => ld_rr_n(Register16Bit::HL),
        0x26 => ld_r_n(Register8Bit::H),
        0x2e => ld_r_n(Register8Bit::L),
        0x31 => ld_rr_n(Register16Bit::SP),
        0x32 => (NoRead(LoadToAddressHlFromADec), vec([NoRead(Nop)])),
        0x3e => ld_r_n(Register8Bit::A),
        0x4f => (
            NoRead(Load {
                to: Register8Bit::C,
                from: Register8Bit::A,
            }),
            Default::default(),
        ),
        0x77 => (
            NoRead(LoadToAddressFromRegister {
                address: Register16Bit::HL,
                value: Register8Bit::A,
            }),
            vec([NoRead(Nop)]),
        ),
        0xaf => (NoRead(Xor(Register8Bit::A)), Default::default()),
        0xc5 => push_rr(Register16Bit::BC),
        0xcb => (NoRead(Nop), Default::default()),
        0xcd => (
            Read(None, ReadIntoLsb),
            vec([
                NoRead(Nop),
                NoRead(WriteLsbPcWhereSpPointsAndLoadCacheToPc),
                NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(None)),
                NoRead(DecStackPointer),
                Read(None, ReadIntoMsb),
            ]),
        ),
        0xd5 => push_rr(Register16Bit::DE),
        0xe0 => (
            Read(None, ReadIntoLsb),
            vec([NoRead(Nop), NoRead(LoadFromAccumulator(None))]),
        ),
        0xe2 => (
            NoRead(LoadFromAccumulator(Some(Register8Bit::C))),
            vec([NoRead(Nop)]),
        ),
        0xe5 => push_rr(Register16Bit::HL),
        0xf5 => push_rr(Register16Bit::AF),
        _ => panic!("Opcode not implemented: 0x{opcode:02x}"),
    }
}

fn get_instructions_cb_mode(opcode: u8) -> Instructions {
    match opcode {
        0x7c => bit_b_r(7, Register8Bit::H),
        0x11 => rl_r(Register8Bit::C),
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
