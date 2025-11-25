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
    LsbSp,
    MsbSp,
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
            Register16Bit::SP => Register8Bit::MsbSp,
            Register16Bit::PC => unreachable!(),
        }
    }

    pub fn get_lsb(self) -> Register8Bit {
        match self {
            Register16Bit::AF => Register8Bit::F,
            Register16Bit::BC => Register8Bit::C,
            Register16Bit::DE => Register8Bit::E,
            Register16Bit::HL => Register8Bit::L,
            Register16Bit::SP => Register8Bit::LsbSp,
            Register16Bit::PC => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Flag {
    Z,
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
    Xor8Bit(Register8Bit),
    Xor,
    // Load to memory HL from A, Decrement
    LoadToAddressHlFromADec,
    LoadToAddressHlFromAInc,
    Bit8Bit(u8, Register8Bit),
    Bit(u8),
    OffsetPc,
    // prefix avec 0xff00
    LoadFromAccumulator(Option<Register8Bit>),
    Inc(Register8Bit),
    Inc16Bit(Register16Bit),
    LoadToAddressFromRegister {
        address: Register16Bit,
        value: Register8Bit,
    },
    LoadToAddressHlN,
    DecStackPointer,
    WriteMsbOfRegisterWhereSpPointsAndDecSp(Register16Bit),
    WriteLsbPcWhereSpPointsAndLoadCacheToPc,
    WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(u16),
    Load {
        to: Register8Bit,
        from: Register8Bit,
    },
    Rl(Register8Bit),
    Srl(Register8Bit),
    Rr(Register8Bit),
    Rra,
    Rla,
    Dec8Bit(Register8Bit),
    Dec16Bit(Register16Bit),
    DecHl,
    Compare,
    Cp8Bit(Register8Bit),
    LoadToCachedAddressFromA,
    Sub8Bit(Register8Bit),
    Sub,
    Add,
    Di,
    Add8Bit(Register8Bit),
    AddHlFirst(Register8Bit),
    AddHlSecond(Register8Bit),
    DecPc,
    Res(u8, Register8Bit),
    ResHl(u8),
    And,
    Or8Bit(Register8Bit),
    Or,
    // besoin d'un refactoring pour lui
    JumpHl,
    Adc,
    ConditionalReturn(Condition),
    SetHl(u8),
    Ei,
    Halt,
    Swap8Bit(Register8Bit),
    LoadHlFromAdjustedStackPointerFirst,
    LoadHlFromAdjustedStackPointerSecond,
    LdSpHl,
    Rlca,
    // https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#nop-and-stop
    Stop,
    WriteLsbSpToCachedAddressAndIncCachedAddress,
    WriteMsbSpToCachedAddress,
    AddSpE,
    Sbc,
}

#[derive(Clone, Copy, Debug)]
pub enum ReadInstruction {
    ReadIntoLsb,
    ReadIntoMsb,
    ConditionalRelativeJump(Condition),
    ConditionalCall(Condition),
    ConditionalJump(Condition),
}

#[derive(Clone, Copy, Debug)]
pub enum OpAfterRead {
    None,
    Inc,
}

#[derive(Clone, Copy, Debug)]
pub enum ReadAddress {
    Register {
        register: Register16Bit,
        op: OpAfterRead,
    },
    // LDH A, (n)
    Accumulator,
    // (nn)
    Cache,
}

const CONSUME_PC: ReadAddress = ReadAddress::Register {
    register: Register16Bit::PC,
    op: OpAfterRead::Inc,
};

pub const POP_SP: ReadAddress = ReadAddress::Register {
    register: Register16Bit::SP,
    op: OpAfterRead::Inc,
};

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    NoRead(NoReadInstruction),
    Read(ReadAddress, ReadInstruction),
}

impl From<NoReadInstruction> for Instruction {
    fn from(value: NoReadInstruction) -> Self {
        Self::NoRead(value)
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Self::NoRead(NoReadInstruction::Nop)
    }
}

#[derive(Debug, Copy, Clone)]
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
    use crate::instructions::CONSUME_PC;
    use crate::instructions::Condition;
    use crate::instructions::OpAfterRead;
    use crate::instructions::POP_SP;
    use crate::instructions::ReadAddress;

    use super::Instruction::*;
    use super::Instructions;
    use super::NoReadInstruction::*;
    use super::ReadInstruction::*;
    use super::Register8Bit;
    use super::Register16Bit;
    use super::vec;

    pub fn ld_r_n(register: Register8Bit) -> Instructions {
        (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Store8Bit(register).into()]),
        )
    }

    pub fn ld_r_r(to: Register8Bit, from: Register8Bit) -> Instructions {
        (Load { to, from }.into(), Default::default())
    }

    pub fn ld_rr_n(register: Register16Bit) -> Instructions {
        (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Store16Bit(register).into(), Read(CONSUME_PC, ReadIntoMsb)]),
        )
    }

    pub fn ld_r_hl(register: Register8Bit) -> Instructions {
        (
            Read(
                ReadAddress::Register {
                    register: Register16Bit::HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Store8Bit(register).into()]),
        )
    }

    pub fn push_rr(register: Register16Bit) -> Instructions {
        (
            DecStackPointer.into(),
            vec([
                Nop.into(),
                LoadToAddressFromRegister {
                    address: Register16Bit::SP,
                    value: register.get_lsb(),
                }
                .into(),
                WriteMsbOfRegisterWhereSpPointsAndDecSp(register).into(),
            ]),
        )
    }

    pub fn bit_b_r(bit: u8, register: Register8Bit) -> Instructions {
        (Bit8Bit(bit, register).into(), Default::default())
    }

    pub fn rl_r(register: Register8Bit) -> Instructions {
        (Rl(register).into(), Default::default())
    }

    pub fn rr_r(register: Register8Bit) -> Instructions {
        (Rr(register).into(), Default::default())
    }

    pub fn srl_r(register: Register8Bit) -> Instructions {
        (Srl(register).into(), Default::default())
    }

    pub fn res_b_r(bit: u8, register: Register8Bit) -> Instructions {
        (Res(bit, register).into(), Default::default())
    }

    pub fn pop_rr(register: Register16Bit) -> Instructions {
        (
            Read(POP_SP, ReadIntoLsb),
            vec([Store16Bit(register).into(), Read(POP_SP, ReadIntoMsb)]),
        )
    }

    pub fn inc_r(register: Register8Bit) -> Instructions {
        (Inc(register).into(), Default::default())
    }

    pub fn inc_rr(register: Register16Bit) -> Instructions {
        (Inc16Bit(register).into(), vec([Nop.into()]))
    }

    pub fn dec_r(register: Register8Bit) -> Instructions {
        (Dec8Bit(register).into(), Default::default())
    }

    pub fn dec_rr(register: Register16Bit) -> Instructions {
        (Dec16Bit(register).into(), vec([Nop.into()]))
    }

    pub fn sub_r(register: Register8Bit) -> Instructions {
        (Sub8Bit(register).into(), Default::default())
    }

    // When there is a jump we have to put a Nop even if the condition will be true
    // or the next opcode will be fetched with the wrong pc
    pub fn jr_cc_e(condition: Condition) -> Instructions {
        (
            Read(CONSUME_PC, ConditionalRelativeJump(condition)),
            vec([Nop.into()]),
        )
    }

    pub fn add_r(register: Register8Bit) -> Instructions {
        (Add8Bit(register).into(), Default::default())
    }

    pub fn or_r(register: Register8Bit) -> Instructions {
        (Or8Bit(register).into(), Default::default())
    }

    pub fn ld_rr_r(address: Register16Bit, value: Register8Bit) -> Instructions {
        (
            LoadToAddressFromRegister { address, value }.into(),
            vec([Nop.into()]),
        )
    }

    pub fn call_cc_nn(condition: Condition) -> Instructions {
        (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Nop.into(), Read(CONSUME_PC, ConditionalCall(condition))]),
        )
    }

    pub fn xor_r(register: Register8Bit) -> Instructions {
        (Xor8Bit(register).into(), Default::default())
    }

    pub fn add_hl_rr(register: Register16Bit) -> Instructions {
        (
            AddHlFirst(register.get_lsb()).into(),
            vec([AddHlSecond(register.get_msb()).into()]),
        )
    }

    pub fn ret_cc(condition: Condition) -> Instructions {
        (ConditionalReturn(condition).into(), vec([Nop.into()]))
    }

    pub fn jp_cc_nn(condition: Condition) -> Instructions {
        (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Nop.into(), Read(CONSUME_PC, ConditionalJump(condition))]),
        )
    }

    pub fn bit_b_hl(bit: u8) -> Instructions {
        (Read(CONSUME_PC, ReadIntoLsb), vec([Bit(bit).into()]))
    }

    pub fn res_b_hl(bit: u8) -> Instructions {
        (
            Read(
                ReadAddress::Register {
                    register: Register16Bit::HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Nop.into(), ResHl(bit).into()]),
        )
    }

    pub fn set_b_hl(bit: u8) -> Instructions {
        (
            Read(
                ReadAddress::Register {
                    register: Register16Bit::HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Nop.into(), SetHl(bit).into()]),
        )
    }

    pub fn swap_r(register: Register8Bit) -> Instructions {
        (Swap8Bit(register).into(), Default::default())
    }

    pub fn cp_r(register: Register8Bit) -> Instructions {
        (Cp8Bit(register).into(), Default::default())
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
        0x07 => (Rlca.into(), Default::default()),
        0x09 => add_hl_rr(BC),
        0x0b => dec_rr(BC),
        0x0c => inc_r(C),
        0x0d => dec_r(C),
        0x0e => ld_r_n(C),
        0x06 => ld_r_n(B),
        0x08 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                Nop.into(),
                WriteMsbSpToCachedAddress.into(),
                WriteLsbSpToCachedAddressAndIncCachedAddress.into(),
                Read(CONSUME_PC, ReadIntoMsb),
            ]),
        ),
        0x10 => {
            println!("stop");
            (Stop.into(), Default::default())
        }
        0x11 => ld_rr_n(DE),
        0x12 => ld_rr_r(DE, A),
        0x13 => inc_rr(DE),
        0x14 => inc_r(D),
        0x15 => dec_r(D),
        0x17 => (Rla.into(), Default::default()),
        0x18 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Nop.into(), OffsetPc.into()]),
        ),
        0x19 => add_hl_rr(DE),
        0x1b => dec_rr(DE),
        0x1c => inc_r(E),
        0x1d => dec_r(E),
        0x1e => ld_r_n(E),
        0x16 => ld_r_n(D),
        0x1a => (
            Read(
                ReadAddress::Register {
                    register: DE,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Store8Bit(A).into()]),
        ),
        0x1f => (Rra.into(), Default::default()),
        0x20 => jr_cc_e(Condition {
            flag: Flag::Z,
            not: true,
        }),
        0x21 => ld_rr_n(HL),
        0x22 => (LoadToAddressHlFromAInc.into(), vec([Nop.into()])),
        0x23 => inc_rr(HL),
        0x24 => inc_r(H),
        0x25 => dec_r(H),
        0x26 => ld_r_n(H),
        0x28 => jr_cc_e(Condition {
            flag: Flag::Z,
            not: false,
        }),
        0x29 => add_hl_rr(HL),
        0x2a => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::Inc,
                },
                ReadIntoLsb,
            ),
            vec([Store8Bit(A).into()]),
        ),
        0x2b => dec_rr(HL),
        0x2c => inc_r(L),
        0x2d => dec_r(L),
        0x2e => ld_r_n(L),
        0x30 => jr_cc_e(Condition {
            flag: Flag::C,
            not: true,
        }),
        0x31 => ld_rr_n(SP),
        0x32 => (LoadToAddressHlFromADec.into(), vec([Nop.into()])),
        0x33 => inc_rr(SP),
        0x35 => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Nop.into(), DecHl.into()]),
        ),
        0x36 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Nop.into(), LoadToAddressHlN.into()]),
        ),
        0x38 => jr_cc_e(Condition {
            flag: Flag::C,
            not: false,
        }),
        0x39 => add_hl_rr(SP),
        0x3b => dec_rr(SP),
        0x3c => inc_r(A),
        0x3d => dec_r(A),
        0x3e => ld_r_n(A),
        0x40 => ld_r_r(B, B),
        0x41 => ld_r_r(B, C),
        0x46 => ld_r_hl(B),
        0x47 => ld_r_r(B, A),
        0x4e => ld_r_hl(C),
        0x4f => ld_r_r(C, A),
        0x56 => ld_r_hl(D),
        0x57 => ld_r_r(D, A),
        0x5d => ld_r_r(E, L),
        0x5e => ld_r_hl(E),
        0x5f => ld_r_r(E, A),
        0x62 => ld_r_r(H, D),
        0x66 => ld_r_hl(H),
        0x67 => ld_r_r(H, A),
        0x6b => ld_r_r(L, E),
        0x6e => ld_r_hl(L),
        0x6f => ld_r_r(L, A),
        0x70 => ld_rr_r(HL, B),
        0x71 => ld_rr_r(HL, C),
        0x72 => ld_rr_r(HL, D),
        0x73 => ld_rr_r(HL, E),
        0x74 => ld_rr_r(HL, H),
        0x75 => ld_rr_r(HL, L),
        0x76 => (Halt.into(), Default::default()),
        0x77 => ld_rr_r(HL, A),
        0x78 => ld_r_r(A, B),
        0x7a => ld_r_r(A, D),
        0x7b => ld_r_r(A, E),
        0x7c => ld_r_r(A, H),
        0x7d => ld_r_r(A, L),
        0x7e => ld_r_hl(A),
        0x79 => ld_r_r(A, C),
        0x80 => add_r(B),
        0x81 => add_r(C),
        0x82 => add_r(D),
        0x83 => add_r(E),
        0x84 => add_r(H),
        0x85 => add_r(L),
        0x86 => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Add.into()]),
        ),
        0x87 => add_r(A),
        0x90 => sub_r(B),
        0x91 => sub_r(C),
        0x92 => sub_r(D),
        0x93 => sub_r(E),
        0x94 => sub_r(H),
        0x95 => sub_r(L),
        0x97 => sub_r(A),
        0xa8 => xor_r(B),
        0xa9 => xor_r(C),
        0xaa => xor_r(D),
        0xab => xor_r(E),
        0xac => xor_r(H),
        0xad => xor_r(L),
        0xae => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Xor.into()]),
        ),
        0xaf => xor_r(A),
        0xb0 => or_r(B),
        0xb1 => or_r(C),
        0xb2 => or_r(D),
        0xb3 => or_r(E),
        0xb4 => or_r(H),
        0xb5 => or_r(L),
        0xb6 => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Or.into()]),
        ),
        0xb7 => or_r(A),
        0xb8 => cp_r(B),
        0xb9 => cp_r(C),
        0xba => cp_r(D),
        0xbb => cp_r(E),
        0xbc => cp_r(H),
        0xbd => cp_r(L),
        0xbe => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Compare.into()]),
        ),
        0xbf => cp_r(A),
        0xc0 => ret_cc(Condition {
            flag: Flag::Z,
            not: true,
        }),
        0xc1 => pop_rr(BC),
        0xc2 => jp_cc_nn(Condition {
            flag: Flag::Z,
            not: true,
        }),
        0xc3 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                Nop.into(),
                Store16Bit(PC).into(),
                Read(CONSUME_PC, ReadIntoMsb),
            ]),
        ),
        0xc4 => call_cc_nn(Condition {
            flag: Flag::Z,
            not: true,
        }),
        0xc5 => push_rr(BC),
        0xc6 => (Read(CONSUME_PC, ReadIntoLsb), vec([Add.into()])),
        0xc8 => ret_cc(Condition {
            flag: Flag::Z,
            not: false,
        }),
        0xc9 => (
            Read(POP_SP, ReadIntoLsb),
            vec([
                Nop.into(),
                Store16Bit(Register16Bit::PC).into(),
                Read(POP_SP, ReadIntoMsb),
            ]),
        ),
        0xca => jp_cc_nn(Condition {
            flag: Flag::Z,
            not: false,
        }),
        0xcb => (Nop.into(), Default::default()),
        0xcc => call_cc_nn(Condition {
            flag: Flag::Z,
            not: false,
        }),
        0xcd => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                Nop.into(),
                WriteLsbPcWhereSpPointsAndLoadCacheToPc.into(),
                WriteMsbOfRegisterWhereSpPointsAndDecSp(PC).into(),
                DecStackPointer.into(),
                Read(CONSUME_PC, ReadIntoMsb),
            ]),
        ),
        0xce => (Read(CONSUME_PC, ReadIntoLsb), vec([Adc.into()])),
        0xd0 => ret_cc(Condition {
            flag: Flag::C,
            not: true,
        }),
        0xd1 => pop_rr(DE),
        0xd2 => jp_cc_nn(Condition {
            flag: Flag::C,
            not: true,
        }),
        0xd4 => call_cc_nn(Condition {
            flag: Flag::C,
            not: true,
        }),
        0xd5 => push_rr(DE),
        0xd6 => (Read(CONSUME_PC, ReadIntoLsb), vec([Sub.into()])),
        0xd8 => ret_cc(Condition {
            flag: Flag::C,
            not: false,
        }),
        0xda => jp_cc_nn(Condition {
            flag: Flag::C,
            not: false,
        }),
        0xdc => call_cc_nn(Condition {
            flag: Flag::C,
            not: false,
        }),
        0xde => (Read(CONSUME_PC, ReadIntoLsb), vec([Sbc.into()])),
        0xe0 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Nop.into(), LoadFromAccumulator(None).into()]),
        ),
        0xe1 => pop_rr(HL),
        0xe2 => (LoadFromAccumulator(Some(C)).into(), vec([Nop.into()])),
        0xe5 => push_rr(HL),
        0xe6 => (Read(CONSUME_PC, ReadIntoLsb), vec([And.into()])),
        // je commence à en avoir marre de détailler chaque opération à chaque cycle.
        // Les changements au niveau des registres n'est pas observable pendant l'exécution
        // d'un opcode donc au final je pense que c'est osef
        0xe8 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Nop.into(), Nop.into(), AddSpE.into()]),
        ),
        0xe9 => (JumpHl.into(), Default::default()),
        0xea => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                Nop.into(),
                LoadToCachedAddressFromA.into(),
                Read(CONSUME_PC, ReadIntoMsb),
            ]),
        ),
        0xee => (Read(CONSUME_PC, ReadIntoLsb), vec([Xor.into()])),
        0xf0 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                Store8Bit(A).into(),
                Read(ReadAddress::Accumulator, ReadIntoLsb),
            ]),
        ),
        0xf1 => pop_rr(AF),
        0xf3 => (Di.into(), Default::default()),
        0xf5 => push_rr(AF),
        0xf6 => (Read(CONSUME_PC, ReadIntoLsb), vec([Or.into()])),
        0xf8 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                LoadHlFromAdjustedStackPointerSecond.into(),
                LoadHlFromAdjustedStackPointerFirst.into(),
            ]),
        ),
        0xf9 => (LdSpHl.into(), vec([Nop.into()])),
        0xfa => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                Store8Bit(A).into(),
                Read(ReadAddress::Cache, ReadIntoLsb),
                Read(CONSUME_PC, ReadIntoMsb),
            ]),
        ),
        0xfb => (Ei.into(), Default::default()),
        0xfe => (Read(CONSUME_PC, ReadIntoLsb), vec([Compare.into()])),
        _ => panic!("Opcode not implemented: 0x{opcode:02x}"),
    }
}

fn get_instructions_cb_mode(opcode: u8) -> Instructions {
    use Register8Bit::*;

    match opcode {
        0x7c => bit_b_r(7, H),
        0x10 => rl_r(B),
        0x11 => rl_r(C),
        0x12 => rl_r(D),
        0x13 => rl_r(E),
        0x14 => rl_r(H),
        0x15 => rl_r(L),
        0x17 => rl_r(A),
        0x18 => rr_r(B),
        0x19 => rr_r(C),
        0x1a => rr_r(D),
        0x1b => rr_r(E),
        0x1c => rr_r(H),
        0x1d => rr_r(L),
        0x1f => rr_r(A),
        0x30 => swap_r(B),
        0x31 => swap_r(C),
        0x32 => swap_r(D),
        0x33 => swap_r(E),
        0x34 => swap_r(H),
        0x35 => swap_r(L),
        0x37 => swap_r(A),
        0x38 => srl_r(B),
        0x39 => srl_r(C),
        0x3a => srl_r(D),
        0x3b => srl_r(E),
        0x3c => srl_r(H),
        0x3d => srl_r(L),
        0x3f => srl_r(A),
        0x46 => bit_b_hl(0),
        0x4e => bit_b_hl(1),
        0x56 => bit_b_hl(2),
        0x5e => bit_b_hl(3),
        0x66 => bit_b_hl(4),
        0x6e => bit_b_hl(5),
        0x76 => bit_b_hl(6),
        0x7e => bit_b_hl(7),
        0x86 => res_b_hl(0),
        0x87 => res_b_r(0, A),
        0x8e => res_b_hl(1),
        0x96 => res_b_hl(2),
        0x9e => res_b_hl(3),
        0xa6 => res_b_hl(4),
        0xae => res_b_hl(5),
        0xb6 => res_b_hl(6),
        0xbe => res_b_hl(7),
        0xc6 => set_b_hl(0),
        0xce => set_b_hl(1),
        0xd6 => set_b_hl(2),
        0xde => set_b_hl(3),
        0xe6 => set_b_hl(4),
        0xee => set_b_hl(5),
        0xf6 => set_b_hl(6),
        0xfe => set_b_hl(7),
        _ => panic!("Opcode not implemented (cb mode): 0x{opcode:02x}"),
    }
}

// une instruction prend plusieurs m-cycles
// l'opcode détermine quel instruction exécuter
// À l'exécution du dernier M-cycle d'une instruction, le prochain opcode est chargé en parallèle
