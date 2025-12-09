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
    W,
    Z,
}

#[derive(Clone, Copy, Debug)]
pub enum Register16Bit {
    AF,
    BC,
    DE,
    SP,
    HL,
    PC,
    WZ,
}

impl Register16Bit {
    pub fn get_msb(self) -> Register8Bit {
        match self {
            Register16Bit::AF => Register8Bit::A,
            Register16Bit::BC => Register8Bit::B,
            Register16Bit::DE => Register8Bit::D,
            Register16Bit::HL => Register8Bit::H,
            Register16Bit::SP => Register8Bit::MsbSp,
            Register16Bit::WZ => Register8Bit::W,
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
            Register16Bit::WZ => Register8Bit::Z,
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
    // Load to memory HL from A, Decrement
    LoadToAddressHlFromADec,
    LoadToAddressHlFromAInc,
    Bit8Bit(u8, Register8Bit),
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
    WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(u8),
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
    Cp8Bit(Register8Bit),
    LoadToCachedAddressFromA,
    Sub8Bit(Register8Bit),
    Di,
    Add8Bit(Register8Bit),
    AddHlFirst(Register8Bit),
    AddHlSecond(Register8Bit),
    DecPc,
    Res(u8, Register8Bit),
    ResHl(u8),
    Or8Bit(Register8Bit),
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
    Reti,
    Cpl,
    Scf,
    Ccf,
    Adc8Bit(Register8Bit),
    Sbc8Bit(Register8Bit),
    And8Bit(Register8Bit),
    Rrca,
    Rlc8Bit(Register8Bit),
    Rrc8Bit(Register8Bit),
    Sla8Bit(Register8Bit),
    Sra8Bit(Register8Bit),
    Set8Bit(u8, Register8Bit),
    RlcHl,
    RrcHl,
    RlHl,
    RrHl,
    SlaHl,
    SraHl,
    SwapHl,
    SrlHl,
    IncHl,
    Daa,
    CbMode,
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
    Dec,
}

#[derive(Clone, Copy, Debug)]
pub enum ReadAddress {
    Register {
        register: Register16Bit,
        op: OpAfterRead,
    },
    // LDH A, (n)
    Accumulator,
    Accumulator8Bit(Register8Bit),
}

impl From<Register16Bit> for ReadAddress {
    fn from(value: Register16Bit) -> Self {
        Self::Register {
            register: value,
            op: OpAfterRead::None,
        }
    }
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

// what to set pc with after the last instruction
pub struct SetPc(pub Register16Bit);

impl Default for SetPc {
    fn default() -> Self {
        Self(Register16Bit::PC)
    }
}

mod opcodes {
    use crate::instructions::CONSUME_PC;
    use crate::instructions::Condition;
    use crate::instructions::NoReadInstruction;
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
        (
            Read(
                ReadAddress::Register {
                    register: Register16Bit::HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Bit8Bit(bit, Register8Bit::Z).into()]),
        )
    }

    // 3 M-cycles (+1 cb opcode)
    fn byte_op_hl(inst: NoReadInstruction) -> Instructions {
        (
            Read(
                ReadAddress::Register {
                    register: Register16Bit::HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Nop.into(), inst.into()]),
        )
    }

    pub fn res_b_hl(bit: u8) -> Instructions {
        byte_op_hl(ResHl(bit))
    }

    pub fn set_b_hl(bit: u8) -> Instructions {
        byte_op_hl(SetHl(bit))
    }

    pub fn swap_r(register: Register8Bit) -> Instructions {
        (Swap8Bit(register).into(), Default::default())
    }

    pub fn cp_r(register: Register8Bit) -> Instructions {
        (Cp8Bit(register).into(), Default::default())
    }

    pub fn rst_n(value: u8) -> Instructions {
        (
            DecStackPointer.into(),
            vec([
                Nop.into(),
                WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(value).into(),
                WriteMsbOfRegisterWhereSpPointsAndDecSp(Register16Bit::PC).into(),
            ]),
        )
    }

    pub fn adc_r(register: Register8Bit) -> Instructions {
        (Adc8Bit(register).into(), Default::default())
    }

    pub fn sbc_r(register: Register8Bit) -> Instructions {
        (Sbc8Bit(register).into(), Default::default())
    }

    pub fn and_r(register: Register8Bit) -> Instructions {
        (And8Bit(register).into(), Default::default())
    }

    pub fn rlc_r(register: Register8Bit) -> Instructions {
        (Rlc8Bit(register).into(), Default::default())
    }

    pub fn rrc_r(register: Register8Bit) -> Instructions {
        (Rrc8Bit(register).into(), Default::default())
    }

    pub fn sla_r(register: Register8Bit) -> Instructions {
        (Sla8Bit(register).into(), Default::default())
    }

    pub fn sra_r(register: Register8Bit) -> Instructions {
        (Sra8Bit(register).into(), Default::default())
    }

    pub fn set_b_r(bit: u8, register: Register8Bit) -> Instructions {
        (Set8Bit(bit, register).into(), Default::default())
    }

    pub fn rlc_hl() -> Instructions {
        byte_op_hl(RlcHl)
    }

    pub fn rrc_hl() -> Instructions {
        byte_op_hl(RrcHl)
    }

    pub fn rl_hl() -> Instructions {
        byte_op_hl(RlHl)
    }

    pub fn rr_hl() -> Instructions {
        byte_op_hl(RrHl)
    }

    pub fn sla_hl() -> Instructions {
        byte_op_hl(SlaHl)
    }

    pub fn sra_hl() -> Instructions {
        byte_op_hl(SraHl)
    }

    pub fn swap_hl() -> Instructions {
        byte_op_hl(SwapHl)
    }

    pub fn srl_hl() -> Instructions {
        byte_op_hl(SrlHl)
    }
}

use opcodes::*;

#[derive(Default)]
pub struct InstructionsAndSetPc(pub Instructions, pub SetPc);

impl From<Instructions> for InstructionsAndSetPc {
    fn from(value: Instructions) -> Self {
        Self(value, Default::default())
    }
}

pub fn get_instructions(opcode: u8, is_cb_mode: bool) -> InstructionsAndSetPc {
    use Instruction::*;
    use NoReadInstruction::*;
    use ReadInstruction::*;
    use Register8Bit::*;
    use Register16Bit::*;

    if is_cb_mode {
        return get_instructions_cb_mode(opcode).into();
    }

    // instructions in arrayvec are reversed
    match opcode {
        0 => Instructions::default(),
        0x01 => ld_rr_n(BC),
        0x02 => (
            LoadToAddressFromRegister {
                address: BC,
                value: A,
            }
            .into(),
            vec([Nop.into()]),
        ),
        0x03 => inc_rr(BC),
        0x04 => inc_r(B),
        0x05 => dec_r(B),
        0x06 => ld_r_n(B),
        0x07 => (Rlca.into(), Default::default()),
        0x09 => add_hl_rr(BC),
        0x0a => (
            Read(
                ReadAddress::Register {
                    register: BC,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Store8Bit(A).into()]),
        ),
        0x0b => dec_rr(BC),
        0x0c => inc_r(C),
        0x0d => dec_r(C),
        0x0e => ld_r_n(C),
        0x08 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                Nop.into(),
                WriteMsbSpToCachedAddress.into(),
                WriteLsbSpToCachedAddressAndIncCachedAddress.into(),
                Read(CONSUME_PC, ReadIntoMsb),
            ]),
        ),
        0x0f => (Rrca.into(), Default::default()),
        0x10 => {
            log::warn!("stop");
            (Stop.into(), Default::default())
        }
        0x11 => ld_rr_n(DE),
        0x12 => ld_rr_r(DE, A),
        0x13 => inc_rr(DE),
        0x14 => inc_r(D),
        0x15 => dec_r(D),
        0x17 => (Rla.into(), Default::default()),
        0x18 => {
            return InstructionsAndSetPc(
                (
                    Read(CONSUME_PC, ReadIntoLsb),
                    vec([Nop.into(), OffsetPc.into()]),
                ),
                SetPc(WZ),
            );
        }
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
        0x27 => (Daa.into(), Default::default()),
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
        0x2f => (Cpl.into(), Default::default()),
        0x30 => jr_cc_e(Condition {
            flag: Flag::C,
            not: true,
        }),
        0x31 => ld_rr_n(SP),
        0x32 => (LoadToAddressHlFromADec.into(), vec([Nop.into()])),
        0x33 => inc_rr(SP),
        0x34 => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Nop.into(), IncHl.into()]),
        ),
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
        0x37 => (Scf.into(), Default::default()),
        0x38 => jr_cc_e(Condition {
            flag: Flag::C,
            not: false,
        }),
        0x39 => add_hl_rr(SP),
        0x3a => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::Dec,
                },
                ReadIntoLsb,
            ),
            vec([Store8Bit(A).into()]),
        ),
        0x3b => dec_rr(SP),
        0x3c => inc_r(A),
        0x3d => dec_r(A),
        0x3e => ld_r_n(A),
        0x3f => (Ccf.into(), Default::default()),
        0x40 => ld_r_r(B, B),
        0x41 => ld_r_r(B, C),
        0x42 => ld_r_r(B, D),
        0x43 => ld_r_r(B, E),
        0x44 => ld_r_r(B, H),
        0x45 => ld_r_r(B, L),
        0x46 => ld_r_hl(B),
        0x47 => ld_r_r(B, A),
        0x48 => ld_r_r(C, B),
        0x49 => ld_r_r(C, C),
        0x4a => ld_r_r(C, D),
        0x4b => ld_r_r(C, E),
        0x4c => ld_r_r(C, H),
        0x4d => ld_r_r(C, L),
        0x4e => ld_r_hl(C),
        0x4f => ld_r_r(C, A),
        0x50 => ld_r_r(D, B),
        0x51 => ld_r_r(D, C),
        0x52 => ld_r_r(D, D),
        0x53 => ld_r_r(D, E),
        0x54 => ld_r_r(D, H),
        0x55 => ld_r_r(D, L),
        0x56 => ld_r_hl(D),
        0x57 => ld_r_r(D, A),
        0x58 => ld_r_r(E, B),
        0x59 => ld_r_r(E, C),
        0x5a => ld_r_r(E, D),
        0x5b => ld_r_r(E, E),
        0x5c => ld_r_r(E, H),
        0x5d => ld_r_r(E, L),
        0x5e => ld_r_hl(E),
        0x5f => ld_r_r(E, A),
        0x60 => ld_r_r(H, B),
        0x61 => ld_r_r(H, C),
        0x62 => ld_r_r(H, D),
        0x63 => ld_r_r(H, E),
        0x64 => ld_r_r(H, H),
        0x65 => ld_r_r(H, L),
        0x66 => ld_r_hl(H),
        0x67 => ld_r_r(H, A),
        0x68 => ld_r_r(L, B),
        0x69 => ld_r_r(L, C),
        0x6a => ld_r_r(L, D),
        0x6b => ld_r_r(L, E),
        0x6c => ld_r_r(L, H),
        0x6d => ld_r_r(L, L),
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
        0x79 => ld_r_r(A, C),
        0x7a => ld_r_r(A, D),
        0x7b => ld_r_r(A, E),
        0x7c => ld_r_r(A, H),
        0x7d => ld_r_r(A, L),
        0x7e => ld_r_hl(A),
        0x7f => ld_r_r(A, A),
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
            vec([Add8Bit(Z).into()]),
        ),
        0x87 => add_r(A),
        0x88 => adc_r(B),
        0x89 => adc_r(C),
        0x8a => adc_r(D),
        0x8b => adc_r(E),
        0x8c => adc_r(H),
        0x8d => adc_r(L),
        0x8e => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Adc8Bit(Z).into()]),
        ),
        0x8f => adc_r(A),
        0x90 => sub_r(B),
        0x91 => sub_r(C),
        0x92 => sub_r(D),
        0x93 => sub_r(E),
        0x94 => sub_r(H),
        0x95 => sub_r(L),
        0x96 => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Sub8Bit(Z).into()]),
        ),
        0x97 => sub_r(A),
        0x98 => sbc_r(B),
        0x99 => sbc_r(C),
        0x9a => sbc_r(D),
        0x9b => sbc_r(E),
        0x9c => sbc_r(H),
        0x9d => sbc_r(L),
        0x9e => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([Sbc8Bit(Z).into()]),
        ),
        0x9f => sbc_r(A),
        0xa0 => and_r(B),
        0xa1 => and_r(C),
        0xa2 => and_r(D),
        0xa3 => and_r(E),
        0xa4 => and_r(H),
        0xa5 => and_r(L),
        0xa6 => (
            Read(
                ReadAddress::Register {
                    register: HL,
                    op: OpAfterRead::None,
                },
                ReadIntoLsb,
            ),
            vec([And8Bit(Z).into()]),
        ),
        0xa7 => and_r(A),
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
            vec([Xor8Bit(Z).into()]),
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
            vec([Or8Bit(Z).into()]),
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
            vec([Cp8Bit(Z).into()]),
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
        0xc6 => (Read(CONSUME_PC, ReadIntoLsb), vec([Add8Bit(Z).into()])),
        0xc7 => rst_n(0),
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
        0xcb => (CbMode.into(), Default::default()),
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
        0xce => (Read(CONSUME_PC, ReadIntoLsb), vec([Adc8Bit(Z).into()])),
        0xcf => rst_n(0x08),
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
        0xd6 => (Read(CONSUME_PC, ReadIntoLsb), vec([Sub8Bit(Z).into()])),
        0xd7 => rst_n(0x10),
        0xd8 => ret_cc(Condition {
            flag: Flag::C,
            not: false,
        }),
        0xd9 => (
            Read(POP_SP, ReadIntoLsb),
            vec([Nop.into(), Reti.into(), Read(POP_SP, ReadIntoMsb)]),
        ),
        0xda => jp_cc_nn(Condition {
            flag: Flag::C,
            not: false,
        }),
        0xdc => call_cc_nn(Condition {
            flag: Flag::C,
            not: false,
        }),
        0xde => (Read(CONSUME_PC, ReadIntoLsb), vec([Sbc8Bit(Z).into()])),
        0xdf => rst_n(0x18),
        0xe0 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Nop.into(), LoadFromAccumulator(None).into()]),
        ),
        0xe1 => pop_rr(HL),
        0xe2 => (LoadFromAccumulator(Some(C)).into(), vec([Nop.into()])),
        0xe5 => push_rr(HL),
        0xe6 => (Read(CONSUME_PC, ReadIntoLsb), vec([And8Bit(Z).into()])),
        0xe7 => rst_n(0x20),
        // je commence à en avoir marre de détailler chaque opération à chaque cycle.
        // Les changements au niveau des registres n'est pas observable pendant l'exécution
        // d'un opcode donc au final je pense que c'est osef
        0xe8 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([Nop.into(), Nop.into(), AddSpE.into()]),
        ),
        0xe9 => return InstructionsAndSetPc(Default::default(), SetPc(HL)),
        0xea => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                Nop.into(),
                LoadToCachedAddressFromA.into(),
                Read(CONSUME_PC, ReadIntoMsb),
            ]),
        ),
        0xee => (Read(CONSUME_PC, ReadIntoLsb), vec([Xor8Bit(Z).into()])),
        0xef => rst_n(0x28),
        0xf0 => (
            Read(CONSUME_PC, ReadIntoLsb),
            vec([
                Store8Bit(A).into(),
                Read(ReadAddress::Accumulator, ReadIntoLsb),
            ]),
        ),
        0xf1 => pop_rr(AF),
        0xf2 => (
            Read(ReadAddress::Accumulator8Bit(C), ReadIntoLsb),
            vec([Store8Bit(A).into()]),
        ),
        0xf3 => (Di.into(), Default::default()),
        0xf5 => push_rr(AF),
        0xf6 => (Read(CONSUME_PC, ReadIntoLsb), vec([Or8Bit(Z).into()])),
        0xf7 => rst_n(0x30),
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
                Read(WZ.into(), ReadIntoLsb),
                Read(CONSUME_PC, ReadIntoMsb),
            ]),
        ),
        0xfb => (Ei.into(), Default::default()),
        0xfe => (Read(CONSUME_PC, ReadIntoLsb), vec([Cp8Bit(Z).into()])),
        0xff => rst_n(0x38),
        _ => panic!("Opcode not implemented: 0x{opcode:02x}"),
    }
    .into()
}

fn get_instructions_cb_mode(opcode: u8) -> Instructions {
    use Register8Bit::*;

    match opcode {
        0x00 => rlc_r(B),
        0x01 => rlc_r(C),
        0x02 => rlc_r(D),
        0x03 => rlc_r(E),
        0x04 => rlc_r(H),
        0x05 => rlc_r(L),
        0x06 => rlc_hl(),
        0x07 => rlc_r(A),
        0x08 => rrc_r(B),
        0x09 => rrc_r(C),
        0x0a => rrc_r(D),
        0x0b => rrc_r(E),
        0x0c => rrc_r(H),
        0x0d => rrc_r(L),
        0x0e => rrc_hl(),
        0x0f => rrc_r(A),
        0x10 => rl_r(B),
        0x11 => rl_r(C),
        0x12 => rl_r(D),
        0x13 => rl_r(E),
        0x14 => rl_r(H),
        0x15 => rl_r(L),
        0x16 => rl_hl(),
        0x17 => rl_r(A),
        0x18 => rr_r(B),
        0x19 => rr_r(C),
        0x1a => rr_r(D),
        0x1b => rr_r(E),
        0x1c => rr_r(H),
        0x1d => rr_r(L),
        0x1e => rr_hl(),
        0x1f => rr_r(A),
        0x20 => sla_r(B),
        0x21 => sla_r(C),
        0x22 => sla_r(D),
        0x23 => sla_r(E),
        0x24 => sla_r(H),
        0x25 => sla_r(L),
        0x26 => sla_hl(),
        0x27 => sla_r(A),
        0x28 => sra_r(B),
        0x29 => sra_r(C),
        0x2a => sra_r(D),
        0x2b => sra_r(E),
        0x2c => sra_r(H),
        0x2d => sra_r(L),
        0x2e => sra_hl(),
        0x2f => sra_r(A),
        0x30 => swap_r(B),
        0x31 => swap_r(C),
        0x32 => swap_r(D),
        0x33 => swap_r(E),
        0x34 => swap_r(H),
        0x35 => swap_r(L),
        0x36 => swap_hl(),
        0x37 => swap_r(A),
        0x38 => srl_r(B),
        0x39 => srl_r(C),
        0x3a => srl_r(D),
        0x3b => srl_r(E),
        0x3c => srl_r(H),
        0x3d => srl_r(L),
        0x3e => srl_hl(),
        0x3f => srl_r(A),
        0x40 => bit_b_r(0, B),
        0x41 => bit_b_r(0, C),
        0x42 => bit_b_r(0, D),
        0x43 => bit_b_r(0, E),
        0x44 => bit_b_r(0, H),
        0x45 => bit_b_r(0, L),
        0x46 => bit_b_hl(0),
        0x47 => bit_b_r(0, A),
        0x48 => bit_b_r(1, B),
        0x49 => bit_b_r(1, C),
        0x4a => bit_b_r(1, D),
        0x4b => bit_b_r(1, E),
        0x4c => bit_b_r(1, H),
        0x4d => bit_b_r(1, L),
        0x4e => bit_b_hl(1),
        0x4f => bit_b_r(1, A),
        0x50 => bit_b_r(2, B),
        0x51 => bit_b_r(2, C),
        0x52 => bit_b_r(2, D),
        0x53 => bit_b_r(2, E),
        0x54 => bit_b_r(2, H),
        0x55 => bit_b_r(2, L),
        0x56 => bit_b_hl(2),
        0x57 => bit_b_r(2, A),
        0x58 => bit_b_r(3, B),
        0x59 => bit_b_r(3, C),
        0x5a => bit_b_r(3, D),
        0x5b => bit_b_r(3, E),
        0x5c => bit_b_r(3, H),
        0x5d => bit_b_r(3, L),
        0x5e => bit_b_hl(3),
        0x5f => bit_b_r(3, A),
        0x60 => bit_b_r(4, B),
        0x61 => bit_b_r(4, C),
        0x62 => bit_b_r(4, D),
        0x63 => bit_b_r(4, E),
        0x64 => bit_b_r(4, H),
        0x65 => bit_b_r(4, L),
        0x66 => bit_b_hl(4),
        0x67 => bit_b_r(4, A),
        0x68 => bit_b_r(5, B),
        0x69 => bit_b_r(5, C),
        0x6a => bit_b_r(5, D),
        0x6b => bit_b_r(5, E),
        0x6c => bit_b_r(5, H),
        0x6d => bit_b_r(5, L),
        0x6e => bit_b_hl(5),
        0x6f => bit_b_r(5, A),
        0x70 => bit_b_r(6, B),
        0x71 => bit_b_r(6, C),
        0x72 => bit_b_r(6, D),
        0x73 => bit_b_r(6, E),
        0x74 => bit_b_r(6, H),
        0x75 => bit_b_r(6, L),
        0x76 => bit_b_hl(6),
        0x77 => bit_b_r(6, A),
        0x78 => bit_b_r(7, B),
        0x79 => bit_b_r(7, C),
        0x7a => bit_b_r(7, D),
        0x7b => bit_b_r(7, E),
        0x7c => bit_b_r(7, H),
        0x7d => bit_b_r(7, L),
        0x7e => bit_b_hl(7),
        0x7f => bit_b_r(7, A),
        0x80 => res_b_r(0, B),
        0x81 => res_b_r(0, C),
        0x82 => res_b_r(0, D),
        0x83 => res_b_r(0, E),
        0x84 => res_b_r(0, H),
        0x85 => res_b_r(0, L),
        0x87 => res_b_r(0, A),
        0x88 => res_b_r(1, B),
        0x89 => res_b_r(1, C),
        0x8a => res_b_r(1, D),
        0x8b => res_b_r(1, E),
        0x8c => res_b_r(1, H),
        0x8d => res_b_r(1, L),
        0x8f => res_b_r(1, A),
        0x90 => res_b_r(2, B),
        0x91 => res_b_r(2, C),
        0x92 => res_b_r(2, D),
        0x93 => res_b_r(2, E),
        0x94 => res_b_r(2, H),
        0x95 => res_b_r(2, L),
        0x97 => res_b_r(2, A),
        0x98 => res_b_r(3, B),
        0x99 => res_b_r(3, C),
        0x9a => res_b_r(3, D),
        0x9b => res_b_r(3, E),
        0x9c => res_b_r(3, H),
        0x9d => res_b_r(3, L),
        0x9f => res_b_r(3, A),
        0xa0 => res_b_r(4, B),
        0xa1 => res_b_r(4, C),
        0xa2 => res_b_r(4, D),
        0xa3 => res_b_r(4, E),
        0xa4 => res_b_r(4, H),
        0xa5 => res_b_r(4, L),
        0xa7 => res_b_r(4, A),
        0xa8 => res_b_r(5, B),
        0xa9 => res_b_r(5, C),
        0xaa => res_b_r(5, D),
        0xab => res_b_r(5, E),
        0xac => res_b_r(5, H),
        0xad => res_b_r(5, L),
        0xaf => res_b_r(5, A),
        0xb0 => res_b_r(6, B),
        0xb1 => res_b_r(6, C),
        0xb2 => res_b_r(6, D),
        0xb3 => res_b_r(6, E),
        0xb4 => res_b_r(6, H),
        0xb5 => res_b_r(6, L),
        0xb7 => res_b_r(6, A),
        0xb8 => res_b_r(7, B),
        0xb9 => res_b_r(7, C),
        0xba => res_b_r(7, D),
        0xbb => res_b_r(7, E),
        0xbc => res_b_r(7, H),
        0xbd => res_b_r(7, L),
        0xbf => res_b_r(7, A),
        0x86 => res_b_hl(0),
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
        0xc0 => set_b_r(0, B),
        0xc1 => set_b_r(0, C),
        0xc2 => set_b_r(0, D),
        0xc3 => set_b_r(0, E),
        0xc4 => set_b_r(0, H),
        0xc5 => set_b_r(0, L),
        0xc7 => set_b_r(0, A),
        0xc8 => set_b_r(1, B),
        0xc9 => set_b_r(1, C),
        0xca => set_b_r(1, D),
        0xcb => set_b_r(1, E),
        0xcc => set_b_r(1, H),
        0xcd => set_b_r(1, L),
        0xcf => set_b_r(1, A),
        0xd0 => set_b_r(2, B),
        0xd1 => set_b_r(2, C),
        0xd2 => set_b_r(2, D),
        0xd3 => set_b_r(2, E),
        0xd4 => set_b_r(2, H),
        0xd5 => set_b_r(2, L),
        0xd7 => set_b_r(2, A),
        0xd8 => set_b_r(3, B),
        0xd9 => set_b_r(3, C),
        0xda => set_b_r(3, D),
        0xdb => set_b_r(3, E),
        0xdc => set_b_r(3, H),
        0xdd => set_b_r(3, L),
        0xdf => set_b_r(3, A),
        0xe0 => set_b_r(4, B),
        0xe1 => set_b_r(4, C),
        0xe2 => set_b_r(4, D),
        0xe3 => set_b_r(4, E),
        0xe4 => set_b_r(4, H),
        0xe5 => set_b_r(4, L),
        0xe7 => set_b_r(4, A),
        0xe8 => set_b_r(5, B),
        0xe9 => set_b_r(5, C),
        0xea => set_b_r(5, D),
        0xeb => set_b_r(5, E),
        0xec => set_b_r(5, H),
        0xed => set_b_r(5, L),
        0xef => set_b_r(5, A),
        0xf0 => set_b_r(6, B),
        0xf1 => set_b_r(6, C),
        0xf2 => set_b_r(6, D),
        0xf3 => set_b_r(6, E),
        0xf4 => set_b_r(6, H),
        0xf5 => set_b_r(6, L),
        0xf7 => set_b_r(6, A),
        0xf8 => set_b_r(7, B),
        0xf9 => set_b_r(7, C),
        0xfa => set_b_r(7, D),
        0xfb => set_b_r(7, E),
        0xfc => set_b_r(7, H),
        0xfd => set_b_r(7, L),
        0xff => set_b_r(7, A),
    }
}

// une instruction prend plusieurs m-cycles
// l'opcode détermine quel instruction exécuter
// À l'exécution du dernier M-cycle d'une instruction, le prochain opcode est chargé en parallèle
