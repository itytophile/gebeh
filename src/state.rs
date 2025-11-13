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
    PC,
}

impl Register16Bit {
    pub fn get_msb(self) -> Register8Bit {
        match self {
            Register16Bit::AF => Register8Bit::A,
            Register16Bit::BC => Register8Bit::B,
            Register16Bit::DE => Register8Bit::D,
            Register16Bit::HL => Register8Bit::H,
            Register16Bit::SP | Register16Bit::PC => unreachable!(),
        }
    }

    pub fn get_lsb(self) -> Register8Bit {
        match self {
            Register16Bit::AF => Register8Bit::F,
            Register16Bit::BC => Register8Bit::C,
            Register16Bit::DE => Register8Bit::E,
            Register16Bit::HL => Register8Bit::L,
            Register16Bit::SP | Register16Bit::PC => unreachable!(),
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
    LoadToAddressHlFromAInc,
    Bit(u8, Register8Bit),
    OffsetPc,
    LoadFromAccumulator(Option<Register8Bit>),
    Inc(Register8Bit),
    Inc16Bit(Register16Bit),
    LoadToAddressFromRegister {
        address: Register16Bit,
        value: Register8Bit,
    },
    DecStackPointer,
    WriteMsbOfRegisterWhereSpPointsAndDecSp(Register16Bit),
    WriteLsbPcWhereSpPointsAndLoadCacheToPc,
    Load {
        to: Register8Bit,
        from: Register8Bit,
    },
    Rl(Register8Bit),
    Rla,
    Dec(Register8Bit),
}

#[derive(Clone, Copy, Debug)]
pub enum ReadInstruction {
    ReadIntoLsb,
    ReadIntoMsb,
    PopStackIntoLsb,
    PopStackIntoMsb,
    RelativeJump(Option<Condition>),
}

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    NoRead(NoReadInstruction),
    Read(Register16Bit, ReadInstruction),
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
    use super::Register16Bit::{self, *};
    use super::vec;

    pub fn ld_r_n(register: Register8Bit) -> Instructions {
        (Read(PC, ReadIntoLsb), vec([NoRead(Store8Bit(register))]))
    }

    pub fn ld_r_r(to: Register8Bit, from: Register8Bit) -> Instructions {
        (NoRead(Load { to, from }), Default::default())
    }

    pub fn ld_rr_n(register: Register16Bit) -> Instructions {
        (
            Read(PC, ReadIntoLsb),
            vec([NoRead(Store16Bit(register)), Read(PC, ReadIntoMsb)]),
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
                NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(register)),
            ]),
        )
    }

    pub fn bit_b_r(bit: u8, register: Register8Bit) -> Instructions {
        (NoRead(Bit(bit, register)), Default::default())
    }

    pub fn rl_r(register: Register8Bit) -> Instructions {
        (NoRead(Rl(register)), Default::default())
    }

    pub fn pop_rr(register: Register16Bit) -> Instructions {
        (
            Read(SP, PopStackIntoLsb),
            vec([NoRead(Store16Bit(register)), Read(SP, PopStackIntoMsb)]),
        )
    }

    pub fn inc_r(register: Register8Bit) -> Instructions {
        (NoRead(Inc(register)), Default::default())
    }

    pub fn inc_rr(register: Register16Bit) -> Instructions {
        (NoRead(Inc16Bit(register)), vec([NoRead(Nop)]))
    }

    pub fn dec_r(register: Register8Bit) -> Instructions {
        (NoRead(Dec(register)), Default::default())
    }
}

use opcodes::*;

pub fn get_instructions(opcode: u8, is_cb_mode: bool) -> Instructions {
    use Instruction::*;
    use NoReadInstruction::*;
    use ReadInstruction::*;
    use Register8Bit::*;
    use Register16Bit::*;

    if is_cb_mode {
        return get_instructions_cb_mode(opcode);
    }

    // instructions in arrayvec are reversed
    match opcode {
        0 => Default::default(),
        0x01 => ld_rr_n(BC),
        0x03 => inc_rr(BC),
        0x04 => inc_r(B),
        0x05 => dec_r(B),
        0x0c => inc_r(C),
        0x0d => dec_r(C),
        0x0e => ld_r_n(C),
        0x06 => ld_r_n(B),
        0x11 => ld_rr_n(DE),
        0x13 => inc_rr(DE),
        0x14 => inc_r(D),
        0x15 => dec_r(D),
        0x17 => (NoRead(Rla), Default::default()),
        0x1c => inc_r(E),
        0x1d => dec_r(E),
        0x1e => ld_r_n(E),
        0x16 => ld_r_n(D),
        0x1a => (Read(DE, ReadIntoLsb), vec([NoRead(Store8Bit(A))])),
        // When there is a jump we have to put a Nop even if the condition will be true
        // or the next opcode will be fetched with the wrong pc
        0x20 => (
            Read(
                PC,
                RelativeJump(Some(Condition {
                    flag: Flag::Z,
                    not: true,
                })),
            ),
            vec([NoRead(Nop)]),
        ),
        0x21 => ld_rr_n(HL),
        0x22 => (NoRead(LoadToAddressHlFromAInc), vec([NoRead(Nop)])),
        0x23 => inc_rr(HL),
        0x24 => inc_r(H),
        0x25 => dec_r(H),
        0x26 => ld_r_n(H),
        0x2c => inc_r(L),
        0x2d => dec_r(L),
        0x2e => ld_r_n(L),
        0x31 => ld_rr_n(SP),
        0x32 => (NoRead(LoadToAddressHlFromADec), vec([NoRead(Nop)])),
        0x33 => inc_rr(SP),
        0x3c => inc_r(A),
        0x3d => dec_r(A),
        0x3e => ld_r_n(A),
        0x4f => ld_r_r(C, A),
        0x77 => (
            NoRead(LoadToAddressFromRegister {
                address: HL,
                value: A,
            }),
            vec([NoRead(Nop)]),
        ),
        0x7b => ld_r_r(A, E),
        0xaf => (NoRead(Xor(A)), Default::default()),
        0xc1 => pop_rr(BC),
        0xc5 => push_rr(BC),
        0xc9 => (
            Read(SP, PopStackIntoLsb),
            vec([
                NoRead(Nop),
                NoRead(Store16Bit(Register16Bit::PC)),
                Read(SP, PopStackIntoMsb),
            ]),
        ),
        0xcb => (NoRead(Nop), Default::default()),
        0xcd => (
            Read(PC, ReadIntoLsb),
            vec([
                NoRead(Nop),
                NoRead(WriteLsbPcWhereSpPointsAndLoadCacheToPc),
                NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(PC)),
                NoRead(DecStackPointer),
                Read(PC, ReadIntoMsb),
            ]),
        ),
        0xd1 => pop_rr(DE),
        0xd5 => push_rr(DE),
        0xe0 => (
            Read(PC, ReadIntoLsb),
            vec([NoRead(Nop), NoRead(LoadFromAccumulator(None))]),
        ),
        0xe1 => pop_rr(HL),
        0xe2 => (NoRead(LoadFromAccumulator(Some(C))), vec([NoRead(Nop)])),
        0xe5 => push_rr(HL),
        0xf1 => pop_rr(AF),
        0xf5 => push_rr(AF),
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
    pub memory: [u8; 0x10000],
}

impl Default for State {
    fn default() -> Self {
        Self {
            instruction_register: Default::default(),
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

    pub fn write(&mut self, address: u16, value: u8) {
        println!("Writing at ${address:04x} with value 0x{value:02x}");
        self.0.memory[usize::from(address)] = value;
    }
}
