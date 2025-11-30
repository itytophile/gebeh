use crate::{
    StateMachine,
    ic::Ints,
    instructions::{
        AfterReadInstruction, Condition, Flag, Instruction, Instructions, InstructionsAndSetPc,
        NoReadInstruction, OpAfterRead, POP_SP, ReadAddress, ReadInstruction, Register8Bit,
        Register16Bit, SetPc, get_instructions, vec,
    },
    state::{MmuWrite, State, WriteOnlyState},
};

use arrayvec::ArrayVec;
use my_lib::HeapSize;

#[derive(HeapSize, Default)]
pub struct Cpu {
    pub sp: u16,
    pub lsb: u8,
    pub msb: u8,
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: Flags,
    pub is_cb_mode: bool,
    pub pc: u16,
    pub instruction_register: (ArrayVec<Instruction, 5>, SetPc),
    pub ime: bool,
    pub is_halted: bool,
    pub interrupt_to_execute: Option<u8>,
    pub stop_mode: bool,
}

impl Cpu {
    fn get_16bit_register(&self, register: Register16Bit) -> u16 {
        match register {
            Register16Bit::AF => u16::from(self.a) << 8 | u16::from(self.f.bits()),
            Register16Bit::BC => u16::from(self.b) << 8 | u16::from(self.c),
            Register16Bit::DE => u16::from(self.d) << 8 | u16::from(self.e),
            Register16Bit::HL => u16::from(self.h) << 8 | u16::from(self.l),
            Register16Bit::WZ => u16::from_be_bytes([self.msb, self.lsb]),
            Register16Bit::SP => self.sp,
            Register16Bit::PC => self.pc,
        }
    }
}

enum PipelineAction {
    Pop,
    Replace(InstructionsAndSetPc),
}

pub fn set_h_add(arg1: u8, arg2: u8) -> bool {
    let lo1 = arg1 & 0x0F;
    let lo2 = arg2 & 0x0F;

    ((lo1 + lo2) & (0x10)) == 0x10
}

pub fn set_h_add_with_carry(arg1: u8, arg2: u8, carry: bool) -> bool {
    let lo1 = arg1 & 0x0F;
    let lo2 = arg2 & 0x0F;

    ((lo1 + lo2 + carry as u8) & (0x10)) == 0x10
}

pub fn set_h_sub(arg1: u8, arg2: u8) -> bool {
    let lo1 = arg1 & 0x0F;
    let lo2 = arg2 & 0x0F;

    (lo1.wrapping_sub(lo2) & (0x10)) == 0x10
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
    pub struct Flags: u8 {
        const Z = 1 << 7;
        const N = 1 << 6;
        const H = 1 << 5;
        const C = 1 << 4;
    }
}

impl CpuWriteOnce<'_> {
    fn get_8bit_register(&self, register: Register8Bit) -> u8 {
        match register {
            Register8Bit::A => self.a.get(),
            Register8Bit::B => self.b.get(),
            Register8Bit::C => self.c.get(),
            Register8Bit::D => self.d.get(),
            Register8Bit::E => self.e.get(),
            Register8Bit::H => self.h.get(),
            Register8Bit::L => self.l.get(),
            Register8Bit::F => self.f.get().bits(),
            Register8Bit::MsbSp => self.sp.get().to_be_bytes()[0],
            Register8Bit::LsbSp => self.sp.get().to_be_bytes()[1],
            Register8Bit::W => self.msb.get(),
            Register8Bit::Z => self.lsb.get(),
        }
    }

    fn set_8bit_register(&mut self, register: Register8Bit, value: u8) {
        match register {
            Register8Bit::A => *self.a.get_mut() = value,
            Register8Bit::B => *self.b.get_mut() = value,
            Register8Bit::C => *self.c.get_mut() = value,
            Register8Bit::D => *self.d.get_mut() = value,
            Register8Bit::E => *self.e.get_mut() = value,
            Register8Bit::H => *self.h.get_mut() = value,
            Register8Bit::L => *self.l.get_mut() = value,
            Register8Bit::F => *self.f.get_mut() = Flags::from_bits_truncate(value),
            Register8Bit::W => *self.msb.get_mut() = value,
            Register8Bit::Z => *self.lsb.get_mut() = value,
            Register8Bit::MsbSp | Register8Bit::LsbSp => unreachable!(),
        }
    }

    fn get_16bit_register(&self, register: Register16Bit) -> u16 {
        match register {
            Register16Bit::AF => u16::from(self.a.get()) << 8 | u16::from(self.f.get().bits()),
            Register16Bit::BC => u16::from(self.b.get()) << 8 | u16::from(self.c.get()),
            Register16Bit::DE => u16::from(self.d.get()) << 8 | u16::from(self.e.get()),
            Register16Bit::HL => u16::from(self.h.get()) << 8 | u16::from(self.l.get()),
            Register16Bit::WZ => u16::from_be_bytes([self.msb.get(), self.lsb.get()]),
            Register16Bit::SP => self.sp.get(),
            Register16Bit::PC => self.pc.get(),
        }
    }

    fn set_16bit_register(&mut self, register: Register16Bit, value: u16) {
        match register {
            Register16Bit::SP => {
                *self.sp.get_mut() = value;
                return;
            }
            Register16Bit::PC => {
                *self.pc.get_mut() = value;
                return;
            }
            _ => {}
        }
        let [msb, lsb] = value.to_be_bytes();
        self.set_8bit_register(register.get_msb(), msb);
        self.set_8bit_register(register.get_lsb(), lsb);
    }

    fn get_flag(&self, flag: Flag) -> bool {
        match flag {
            Flag::Z => self.f.get().contains(Flags::Z),
            Flag::C => self.f.get().contains(Flags::C),
        }
    }

    fn execute_instruction(&mut self, mut mmu: MmuWrite, inst: AfterReadInstruction) {
        use AfterReadInstruction::*;
        use NoReadInstruction::*;
        use ReadInstruction::*;

        match inst {
            Read(value, inst) => {
                match inst {
                    ReadIntoLsb => *self.lsb.get_mut() = value,
                    ReadIntoMsb => *self.msb.get_mut() = value,
                    ConditionalRelativeJump(Condition { flag, not }) => {
                        *self.lsb.get_mut() = value;
                        if self.get_flag(flag) != not {
                            *self.instruction_register.get_mut() = (
                                vec([Instruction::NoRead(Nop), Instruction::NoRead(OffsetPc)]),
                                SetPc(Register16Bit::WZ),
                            );
                        }
                    }
                    ConditionalCall(Condition { flag, not }) => {
                        *self.msb.get_mut() = value; // msb not like jr
                        if self.get_flag(flag) != not {
                            *self.instruction_register.get_mut() = (
                                vec([
                                    Nop.into(),
                                    WriteLsbPcWhereSpPointsAndLoadCacheToPc.into(),
                                    WriteMsbOfRegisterWhereSpPointsAndDecSp(Register16Bit::PC)
                                        .into(),
                                    DecStackPointer.into(),
                                ]),
                                Default::default(),
                            );
                        }
                    }
                    ConditionalJump(Condition { flag, not }) => {
                        *self.msb.get_mut() = value; // msb not like jr
                        if self.get_flag(flag) != not {
                            *self.instruction_register.get_mut() = (
                                vec([Nop.into(), Store16Bit(Register16Bit::PC).into()]),
                                Default::default(),
                            );
                        }
                    }
                }
            }
            NoRead(Nop) => {}
            NoRead(Store8Bit(register)) => {
                self.set_8bit_register(register, self.lsb.get());
            }
            NoRead(Store16Bit(register)) => {
                self.set_16bit_register(
                    register,
                    u16::from_be_bytes([self.msb.get(), self.lsb.get()]),
                );
            }
            NoRead(Xor8Bit(register)) => {
                *self.a.get_mut() ^= self.get_8bit_register(register);
                let mut flags = self.f.get();
                flags.set(Flags::Z, self.a.get() == 0);
                flags.remove(Flags::N);
                flags.remove(Flags::H);
                flags.remove(Flags::C);
                *self.f.get_mut() = flags;
            }
            NoRead(LoadToAddressHlFromADec) => {
                let hl = self.get_16bit_register(Register16Bit::HL);
                mmu.write(hl, self.a.get());
                [*self.h.get_mut(), *self.l.get_mut()] = hl.wrapping_sub(1).to_be_bytes();
            }
            NoRead(LoadToAddressHlFromAInc) => {
                let hl = self.get_16bit_register(Register16Bit::HL);
                mmu.write(hl, self.a.get());
                [*self.h.get_mut(), *self.l.get_mut()] = hl.wrapping_add(1).to_be_bytes();
            }
            NoRead(Bit8Bit(bit, register)) => {
                let mut flags = self.f.get();
                flags.set(
                    Flags::Z,
                    (self.get_8bit_register(register) & (1 << bit)) == 0,
                );
                flags.remove(Flags::N);
                flags.insert(Flags::H);
                *self.f.get_mut() = flags;
            }
            NoRead(OffsetPc) => {
                self.set_16bit_register(
                    Register16Bit::WZ,
                    self.pc
                        .get()
                        .wrapping_add_signed(i16::from(self.lsb.get().cast_signed())),
                );
            }
            NoRead(LoadFromAccumulator(register)) => {
                mmu.write(
                    0xff00
                        | u16::from(
                            register
                                .map(|register| self.get_8bit_register(register))
                                .unwrap_or(self.lsb.get()),
                        ),
                    self.a.get(),
                );
            }
            NoRead(Inc(register)) => {
                let incremented = self.inc(self.get_8bit_register(register));
                self.set_8bit_register(register, incremented);
            }
            NoRead(Inc16Bit(register)) => {
                self.set_16bit_register(register, self.get_16bit_register(register).wrapping_add(1))
            }
            NoRead(LoadToAddressFromRegister { address, value }) => {
                mmu.write(
                    self.get_16bit_register(address),
                    self.get_8bit_register(value),
                );
            }
            NoRead(DecStackPointer) => {
                *self.sp.get_mut() = self.sp.get().wrapping_sub(1);
            }
            NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(register)) => {
                mmu.write(
                    self.sp.get(),
                    self.get_16bit_register(register).to_be_bytes()[0],
                );
                *self.sp.get_mut() = self.sp.get().wrapping_sub(1);
            }
            NoRead(WriteLsbPcWhereSpPointsAndLoadCacheToPc) => {
                mmu.write(self.sp.get(), self.pc.get().to_be_bytes()[1]);
                *self.pc.get_mut() = u16::from_be_bytes([self.msb.get(), self.lsb.get()]);
            }
            NoRead(Load { to, from }) => {
                self.set_8bit_register(to, self.get_8bit_register(from));
            }
            NoRead(Rl(register)) => {
                let result = self.rl(self.get_8bit_register(register));
                self.set_8bit_register(register, result);
            }
            NoRead(Srl(register)) => {
                let result = self.srl(self.get_8bit_register(register));
                self.set_8bit_register(register, result);
            }
            NoRead(Rr(register)) => {
                let result = self.rr(self.get_8bit_register(register));
                self.set_8bit_register(register, result);
            }
            NoRead(Rra) => {
                let value = self.a.get();
                let mut flags = self.f.get();
                let carry = flags.contains(Flags::C);
                flags.set(Flags::C, (value & 0x1) == 0x1);
                let result = (value >> 1) | ((carry as u8) << 7);
                *self.a.get_mut() = result;
                flags.remove(Flags::N);
                flags.remove(Flags::Z); // difference with rr_r
                flags.remove(Flags::H);
                *self.f.get_mut() = flags;
            }
            NoRead(Rla) => {
                let new_carry = (self.a.get() & 0x80) == 0x80;
                *self.a.get_mut() = (self.a.get() << 1) | (self.f.get().contains(Flags::C) as u8);
                let mut flags = self.f.get();
                flags.remove(Flags::Z); // difference with rl r
                flags.remove(Flags::N);
                flags.remove(Flags::H);
                flags.set(Flags::C, new_carry);
                *self.f.get_mut() = flags;
            }
            NoRead(Dec8Bit(register)) => {
                let r = self.get_8bit_register(register);
                let decremented = r.wrapping_sub(1);
                self.set_8bit_register(register, decremented);
                let mut flags = self.f.get();
                flags.set(Flags::Z, decremented == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, set_h_sub(r, 1));
                *self.f.get_mut() = flags;
            }
            NoRead(DecHl) => {
                let r = self.lsb.get();
                let decremented = r.wrapping_sub(1);
                mmu.write(self.get_16bit_register(Register16Bit::HL), decremented);
                let mut flags = self.f.get();
                flags.set(Flags::Z, decremented == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, set_h_sub(r, 1));
                *self.f.get_mut() = flags;
            }
            NoRead(LoadToCachedAddressFromA) => {
                mmu.write(
                    u16::from_be_bytes([self.msb.get(), self.lsb.get()]),
                    self.a.get(),
                );
            }
            NoRead(Sub8Bit(register)) => {
                let a = self.a.get();
                let r = self.get_8bit_register(register);
                let (result, carry) = a.overflowing_sub(r);
                let mut flags = self.f.get();
                flags.set(Flags::Z, result == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, set_h_sub(a, r));
                flags.set(Flags::C, carry);
                *self.f.get_mut() = flags;
                *self.a.get_mut() = result;
            }
            NoRead(Add8Bit(register)) => {
                let a = self.a.get();
                let register_value = self.get_8bit_register(register);
                let (result, carry) = a.overflowing_add(register_value);
                let mut flags = self.f.get();
                flags.set(Flags::Z, result == 0);
                flags.remove(Flags::N);
                flags.set(Flags::H, set_h_add(a, register_value));
                flags.set(Flags::C, carry);
                *self.f.get_mut() = flags;
                *self.a.get_mut() = result;
            }
            NoRead(Di) => {
                *self.ime.get_mut() = false;
            }
            NoRead(Ei) => {
                *self.ime.get_mut() = true;
            }
            NoRead(DecPc) => {
                *self.pc.get_mut() -= 1;
            }
            NoRead(WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(address)) => {
                mmu.write(self.sp.get(), self.pc.get().to_be_bytes()[1]);
                *self.pc.get_mut() = u16::from(address);
            }
            NoRead(Res(bit, register)) => {
                self.set_8bit_register(register, self.get_8bit_register(register) & !(1 << bit));
            }
            NoRead(ResHl(bit)) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.lsb.get() & !(1 << bit),
                );
            }
            NoRead(LoadToAddressHlN) => {
                mmu.write(self.get_16bit_register(Register16Bit::HL), self.lsb.get());
            }
            NoRead(Dec16Bit(register)) => {
                self.set_16bit_register(
                    register,
                    self.get_16bit_register(register).wrapping_sub(1),
                );
            }
            NoRead(Or8Bit(register)) => {
                let result = self.a.get() | self.get_8bit_register(register);
                *self.a.get_mut() = result;
                let mut flags = self.f.get();
                flags.set(Flags::Z, result == 0);
                flags.remove(Flags::N);
                flags.remove(Flags::H);
                flags.remove(Flags::C);
                *self.f.get_mut() = flags;
            }
            NoRead(AddHlFirst(register)) => {
                let l = self.l.get();
                let register_value = self.get_8bit_register(register);
                let (result, carry) = l.overflowing_add(register_value);
                let mut flags = self.f.get();
                // no z
                flags.remove(Flags::N);
                flags.set(Flags::H, set_h_add(l, register_value));
                flags.set(Flags::C, carry);
                *self.f.get_mut() = flags;
                *self.l.get_mut() = result;
            }
            NoRead(AddHlSecond(register)) => {
                let h = self.h.get();
                let register_value = self.get_8bit_register(register);
                let (result, mut carry) = h.overflowing_add(register_value);
                let mut flags = self.f.get();
                let (result, carry1) = result.overflowing_add(flags.contains(Flags::C) as u8);
                carry |= carry1;
                // no z
                flags.remove(Flags::N);
                flags.set(
                    Flags::H,
                    set_h_add_with_carry(h, register_value, flags.contains(Flags::C)),
                );
                flags.set(Flags::C, carry);
                *self.f.get_mut() = flags;
                *self.h.get_mut() = result;
            }
            NoRead(ConditionalReturn(Condition { flag, not })) => {
                if self.get_flag(flag) != not {
                    *self.instruction_register.get_mut() = (
                        vec([
                            Nop.into(),
                            Store16Bit(Register16Bit::PC).into(),
                            Instruction::Read(POP_SP, ReadIntoMsb),
                            Instruction::Read(POP_SP, ReadIntoLsb),
                        ]),
                        Default::default(),
                    );
                }
            }
            NoRead(SetHl(bit)) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.lsb.get() | (1 << bit),
            ),
            NoRead(Halt) => {
                *self.is_halted.get_mut() = true;
            }
            NoRead(Swap8Bit(register)) => {
                let result = self.swap(self.get_8bit_register(register));
                self.set_8bit_register(register, result);
            }
            NoRead(LoadHlFromAdjustedStackPointerFirst) => {
                let [_, sp_lsb] = self.sp.get().to_be_bytes();
                let (result, carry) = sp_lsb.overflowing_add(self.lsb.get());
                *self.l.get_mut() = result;
                let mut flags = self.f.get();
                flags.remove(Flags::Z);
                flags.remove(Flags::N);
                flags.set(Flags::H, set_h_add(sp_lsb, self.lsb.get()));
                flags.set(Flags::C, carry);
                *self.f.get_mut() = flags;
            }
            NoRead(LoadHlFromAdjustedStackPointerSecond) => {
                [*self.h.get_mut(), _] = self
                    .sp
                    .get()
                    .wrapping_add_signed(i16::from(self.lsb.get().cast_signed()))
                    .to_be_bytes();
            }
            NoRead(Cp8Bit(register)) => {
                let a = self.a.get();
                let value = self.get_8bit_register(register);
                let (result, carry) = a.overflowing_sub(value);
                let mut flags = self.f.get();
                flags.set(Flags::Z, result == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, set_h_sub(a, value));
                flags.set(Flags::C, carry);
                *self.f.get_mut() = flags;
            }
            NoRead(LdSpHl) => {
                *self.sp.get_mut() = self.get_16bit_register(Register16Bit::HL);
            }
            NoRead(Rlca) => {
                let a = self.a.get();
                let carry = a >> 7;
                *self.a.get_mut() = (a << 1) | carry;
                let mut flags = self.f.get();
                flags.remove(Flags::Z);
                flags.remove(Flags::N);
                flags.remove(Flags::H);
                flags.set(Flags::C, carry == 1);
                *self.f.get_mut() = flags;
            }
            NoRead(Stop) => {
                *self.stop_mode.get_mut() = true;
            }
            NoRead(WriteLsbSpToCachedAddressAndIncCachedAddress) => {
                let [_, lsb] = self.sp.get().to_be_bytes();
                let wz = u16::from_be_bytes([self.msb.get(), self.lsb.get()]);
                mmu.write(wz, lsb);
                [*self.msb.get_mut(), *self.lsb.get_mut()] = wz.wrapping_add(1).to_be_bytes();
            }
            NoRead(WriteMsbSpToCachedAddress) => {
                let [msb, _] = self.sp.get().to_be_bytes();
                mmu.write(u16::from_be_bytes([self.msb.get(), self.lsb.get()]), msb);
            }
            NoRead(AddSpE) => {
                let [_, sp_lsb] = self.sp.get().to_be_bytes();
                let (_, carry) = sp_lsb.overflowing_add(self.lsb.get());
                let mut flags = self.f.get();
                flags.remove(Flags::Z);
                flags.remove(Flags::N);
                flags.set(Flags::H, set_h_add(sp_lsb, self.lsb.get()));
                flags.set(Flags::C, carry);
                *self.f.get_mut() = flags;
                *self.sp.get_mut() = self
                    .sp
                    .get()
                    .wrapping_add_signed(i16::from(self.lsb.get().cast_signed()));
            }
            NoRead(Reti) => {
                *self.pc.get_mut() = u16::from_be_bytes([self.msb.get(), self.lsb.get()]);
                *self.ime.get_mut() = true;
            }
            NoRead(Cpl) => {
                self.f.get_mut().insert(Flags::N | Flags::H);
                *self.a.get_mut() = !self.a.get();
            }
            NoRead(Scf) => {
                let flags = self.f.get_mut();
                flags.remove(Flags::N | Flags::H);
                flags.insert(Flags::C);
            }
            NoRead(Ccf) => {
                let flags = self.f.get_mut();
                flags.remove(Flags::N | Flags::H);
                flags.toggle(Flags::C);
            }
            NoRead(Adc8Bit(register)) => {
                self.adc(self.get_8bit_register(register));
            }
            NoRead(Sbc8Bit(register)) => {
                self.sbc(self.get_8bit_register(register));
            }
            NoRead(And8Bit(register)) => {
                let result = self.a.get() & self.get_8bit_register(register);
                *self.a.get_mut() = result;
                let mut flags = self.f.get();
                flags.set(Flags::Z, result == 0);
                flags.remove(Flags::N);
                flags.insert(Flags::H);
                flags.remove(Flags::C);
                *self.f.get_mut() = flags;
            }
            NoRead(Rrca) => {
                let a = self.a.get();
                *self.a.get_mut() = a.rotate_right(1);
                let flags = self.f.get_mut();
                flags.remove(Flags::Z | Flags::N | Flags::H);
                flags.set(Flags::C, (a & 1) == 1)
            }
            NoRead(Rlc8Bit(register)) => {
                let result = self.rlc(self.get_8bit_register(register));
                self.set_8bit_register(register, result);
            }
            NoRead(Rrc8Bit(register)) => {
                let result = self.rrc(self.get_8bit_register(register));
                self.set_8bit_register(register, result);
            }
            NoRead(Sla8Bit(register)) => {
                let result = self.sla(self.get_8bit_register(register));
                self.set_8bit_register(register, result);
            }
            NoRead(Sra8Bit(register)) => {
                let result = self.sra(self.get_8bit_register(register));
                self.set_8bit_register(register, result);
            }
            NoRead(Set8Bit(bit, register)) => {
                self.set_8bit_register(register, self.get_8bit_register(register) | (1 << bit));
            }
            NoRead(RlcHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.rlc(self.lsb.get()),
            ),
            NoRead(RrcHl) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.rrc(self.lsb.get()),
                );
            }
            NoRead(RlHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.rl(self.lsb.get()),
            ),
            NoRead(RrHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.rr(self.lsb.get()),
            ),
            NoRead(SlaHl) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.sla(self.lsb.get()),
                );
            }
            NoRead(SraHl) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.sra(self.lsb.get()),
                );
            }
            NoRead(SwapHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.swap(self.lsb.get()),
            ),
            NoRead(SrlHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.srl(self.lsb.get()),
            ),
            NoRead(IncHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.inc(self.lsb.get()),
            ),
            NoRead(Daa) => {
                // https://rgbds.gbdev.io/docs/v1.0.0/gbz80.7#DAA
                let flags = self.f.get_mut();
                let mut adj = 0;
                let result = if flags.contains(Flags::N) {
                    if flags.contains(Flags::H) {
                        adj += 0x06
                    }
                    if flags.contains(Flags::C) {
                        adj += 0x60
                    }
                    self.a.get().wrapping_sub(adj)
                } else {
                    let a = self.a.get();
                    if flags.contains(Flags::H) || (a & 0x0f) > 0x09 {
                        adj += 0x06;
                    }
                    if flags.contains(Flags::C) || a > 0x99 {
                        adj += 0x60;
                        flags.insert(Flags::C);
                    }
                    self.a.get().wrapping_add(adj)
                };
                *self.a.get_mut() = result;
                flags.set(Flags::Z, result == 0);
                flags.remove(Flags::H);
            }
        }
    }

    fn adc(&mut self, second: u8) {
        let first = self.a.get() as u32;
        let second = second as u32;
        let flags = self.f.get_mut();
        let carry = flags.contains(Flags::C) as u32;

        let result = first.wrapping_add(second).wrapping_add(carry);
        let result_b = result as u8;

        flags.remove(Flags::N);
        flags.set(Flags::Z, result_b == 0);
        flags.set(Flags::H, (first ^ second ^ result) & 0x10 == 0x10);
        flags.set(Flags::C, (result & 0x100) == 0x100);

        *self.a.get_mut() = result_b;
    }

    fn sbc(&mut self, second: u8) {
        let first = self.a.get() as u32;
        let second = second as u32;
        let flags = self.f.get_mut();
        let carry = flags.contains(Flags::C) as u32;

        let result = first.wrapping_sub(second).wrapping_sub(carry);
        let result_b = result as u8;

        flags.insert(Flags::N);
        flags.set(Flags::Z, result_b == 0);
        flags.set(Flags::H, (first ^ second ^ result) & 0x10 == 0x10);
        flags.set(Flags::C, (result & 0x100) == 0x100);

        *self.a.get_mut() = result_b;
    }

    fn sla(&mut self, value: u8) -> u8 {
        let result = value << 1;
        let flags = self.f.get_mut();
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (value & 0x80) == 0x80);
        result
    }

    fn sra(&mut self, value: u8) -> u8 {
        let result = (value >> 1) | (value & 0x80);
        let flags = self.f.get_mut();
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (value & 0x1) == 0x1);
        result
    }

    fn inc(&mut self, value: u8) -> u8 {
        let incremented = value.wrapping_add(1);
        let flags = self.f.get_mut();
        flags.set(Flags::Z, incremented == 0);
        flags.remove(Flags::N);
        flags.set(Flags::H, set_h_add(value, 1));
        incremented
    }

    fn rrc(&mut self, value: u8) -> u8 {
        let flags = self.f.get_mut();
        flags.set(Flags::Z, value == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (value & 1) == 1);
        value.rotate_right(1)
    }

    fn rlc(&mut self, value: u8) -> u8 {
        let result = value.rotate_left(1);
        let flags = self.f.get_mut();
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (result & 1) == 1);
        result
    }

    fn rl(&mut self, value: u8) -> u8 {
        let new_carry = (value & 0x80) == 0x80;
        let result = (value << 1) | (self.f.get().contains(Flags::C) as u8);
        let flags = self.f.get_mut();
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, new_carry);
        result
    }

    fn srl(&mut self, value: u8) -> u8 {
        let result = value >> 1;
        let flags = self.f.get_mut();
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (value & 0x1) == 0x1);
        result
    }

    fn rr(&mut self, value: u8) -> u8 {
        let flags = self.f.get_mut();
        let carry = flags.contains(Flags::C);
        flags.set(Flags::C, (value & 0x1) == 0x1);
        let result = (value >> 1) | ((carry as u8) << 7);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::Z, result == 0);
        result
    }

    fn swap(&mut self, value: u8) -> u8 {
        let result = ((value >> 4) & 0x0f) | ((value << 4) & 0xf0);
        let flags = self.f.get_mut();
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H | Flags::C);
        result
    }
}

impl StateMachine for Cpu {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        let interrupts_to_execute = state.interrupt_enable & state.interrupt_flag;

        // https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#nop-and-stop
        if self.stop_mode {
            self.stop_mode = false;
            // quand on va sortir du stop mode on va exécuter un nop
            // et fetch le prochain opcode en parallèle
            self.instruction_register = Default::default();
            // permet de passer un cycle en stop mode sans rien faire
            return None;
        }

        // https://gbdev.io/pandocs/halt.html#halt
        if self.is_halted && !interrupts_to_execute.is_empty() {
            self.is_halted = false;
        }

        if self.is_halted {
            return None;
        }

        let mmu = state.mmu();

        let mut interrupt_flag_to_reset = Option::<Ints>::None;

        // https://gbdev.io/pandocs/Interrupt_Sources.html
        // interrupt_to_execute peut être défini en même temps que ime = true
        // dans le cas du RETI
        // https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#ret-and-reti
        // Malheureusement je ne comprends pas l'explication, donc je vais simplement
        // désactiver la vérification des interruptions tant que interrupt_to_execute est défini
        // Pas de write_once pour les interruptions car c'est trop spécifique (oui raison de merde)
        if self.interrupt_to_execute.is_none()
            && self.ime
            && let Some((interrupt, address)) = [
                (Ints::VBLANK, 0x40),
                (Ints::LCD, 0x48),
                (Ints::TIMER, 0x50),
                (Ints::SERIAL, 0x58),
                (Ints::JOYPAD, 0x60),
            ]
            .into_iter()
            .find(|(flag, _)| interrupts_to_execute.contains(*flag))
        {
            println!("Interrupt handler: {interrupt:?}");
            // Citation: The IF bit corresponding to this interrupt and the IME flag are reset by the CPU
            // https://gbdev.io/pandocs/Interrupts.html#interrupt-handling
            interrupt_flag_to_reset = Some(interrupt);
            self.ime = false;
            // interrupt will be handled at next opcode
            // Citation: and interrupt servicing happens after fetching the next opcode,
            // so PC has to be adjusted to point to the next executed instruction
            // https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#isr-and-nmi
            self.interrupt_to_execute = Some(address);
        }

        let inst = if let Some(inst) = self.instruction_register.0.pop() {
            inst
        } else {
            // affecter et incrémenter le pc même dans le cas de l'interruption
            self.pc = self.get_16bit_register(self.instruction_register.1.0);
            let opcode = mmu.read(self.pc);
            self.pc = self.pc.wrapping_add(1);
            if let Some(address) = self.interrupt_to_execute.take() {
                println!("Interrupt handling");
                use NoReadInstruction::*;
                self.instruction_register.0 = vec([
                    Nop.into(),
                    WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(address).into(),
                    WriteMsbOfRegisterWhereSpPointsAndDecSp(Register16Bit::PC).into(),
                    DecStackPointer.into(),
                ]);
                self.instruction_register.1 = Default::default();
                DecPc.into()
            } else {
                let prout = get_instructions(opcode, self.is_cb_mode);
                self.is_cb_mode = !self.is_cb_mode && opcode == 0xcb;
                self.instruction_register.0 = prout.0.1;
                self.instruction_register.1 = prout.1;
                prout.0.0
            }
        };

        let mut write_once = self.write_once();

        let inst = match inst {
            Instruction::NoRead(no_read) => AfterReadInstruction::NoRead(no_read),
            Instruction::Read(ReadAddress::Accumulator, inst) => {
                AfterReadInstruction::Read(mmu.read(0xff00 | u16::from(write_once.lsb.get())), inst)
            }
            Instruction::Read(ReadAddress::Accumulator8Bit(register), inst) => {
                AfterReadInstruction::Read(
                    mmu.read(0xff00 | u16::from(write_once.get_8bit_register(register))),
                    inst,
                )
            }
            Instruction::Read(ReadAddress::Register { register, op }, inst) => {
                let register_value = write_once.get_16bit_register(register);
                match op {
                    OpAfterRead::None => {}
                    OpAfterRead::Inc => {
                        write_once.set_16bit_register(register, register_value.wrapping_add(1));
                    }
                    OpAfterRead::Dec => {
                        write_once.set_16bit_register(register, register_value.wrapping_sub(1));
                    }
                }
                AfterReadInstruction::Read(mmu.read(register_value), inst)
            }
        };

        Some(move |mut state: WriteOnlyState<'_>| {
            if let Some(flag) = interrupt_flag_to_reset {
                state.remove_if_bit(flag);
            }

            write_once.execute_instruction(state.mmu(), inst);

            // https://gbdev.io/pandocs/halt.html#halt-bug
            if let AfterReadInstruction::NoRead(NoReadInstruction::Halt) = inst
                && !write_once.ime.get()
                && !interrupts_to_execute.is_empty()
            {
                todo!("halt bug")
            }
        })
    }
}
