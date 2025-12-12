use crate::{
    StateMachine,
    ic::Ints,
    instructions::{
        AfterReadInstruction, Condition, Flag, Instruction, InstructionsAndSetPc,
        NoReadInstruction, OpAfterRead, POP_SP, ReadAddress, ReadInstruction, Register8Bit,
        Register16Bit, SetPc, get_instructions, vec,
    },
    state::{State, WriteOnlyState},
};

use arrayvec::ArrayVec;

#[derive(Clone)]
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

impl Default for Cpu {
    fn default() -> Self {
        Self {
            sp: Default::default(),
            lsb: Default::default(),
            msb: Default::default(),
            a: Default::default(),
            b: Default::default(),
            c: Default::default(),
            d: Default::default(),
            e: Default::default(),
            h: Default::default(),
            l: Default::default(),
            f: Default::default(),
            is_cb_mode: Default::default(),
            pc: Default::default(),
            // yes the cpu can fetch opcodes in parallel of the execution but for the first boost we must
            // feed a nop or the cpu will fetch + execute the fist opcode in the same cycle
            instruction_register: (vec([NoReadInstruction::Nop.into()]), Default::default()),
            ime: Default::default(),
            is_halted: Default::default(),
            interrupt_to_execute: Default::default(),
            stop_mode: Default::default(),
        }
    }
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

impl Cpu {
    fn get_8bit_register(&self, register: Register8Bit) -> u8 {
        match register {
            Register8Bit::A => self.a,
            Register8Bit::B => self.b,
            Register8Bit::C => self.c,
            Register8Bit::D => self.d,
            Register8Bit::E => self.e,
            Register8Bit::H => self.h,
            Register8Bit::L => self.l,
            Register8Bit::F => self.f.bits(),
            Register8Bit::MsbSp => self.sp.to_be_bytes()[0],
            Register8Bit::LsbSp => self.sp.to_be_bytes()[1],
            Register8Bit::W => self.msb,
            Register8Bit::Z => self.lsb,
        }
    }

    fn set_8bit_register(&mut self, register: Register8Bit, value: u8) {
        match register {
            Register8Bit::A => self.a = value,
            Register8Bit::B => self.b = value,
            Register8Bit::C => self.c = value,
            Register8Bit::D => self.d = value,
            Register8Bit::E => self.e = value,
            Register8Bit::H => self.h = value,
            Register8Bit::L => self.l = value,
            Register8Bit::F => self.f = Flags::from_bits_truncate(value),
            Register8Bit::W => self.msb = value,
            Register8Bit::Z => self.lsb = value,
            Register8Bit::MsbSp | Register8Bit::LsbSp => unreachable!(),
        }
    }

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

    fn set_16bit_register(&mut self, register: Register16Bit, value: u16) {
        match register {
            Register16Bit::SP => {
                self.sp = value;
                return;
            }
            Register16Bit::PC => {
                self.pc = value;
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
            Flag::Z => self.f.contains(Flags::Z),
            Flag::C => self.f.contains(Flags::C),
        }
    }

    fn execute_instruction(&mut self, mut state: WriteOnlyState, inst: AfterReadInstruction) {
        use AfterReadInstruction::*;
        use NoReadInstruction::*;
        use ReadInstruction::*;

        let mut mmu = state.mmu();

        match inst {
            Read(value, inst) => {
                match inst {
                    ReadIntoLsb => self.lsb = value,
                    ReadIntoMsb => self.msb = value,
                    ConditionalRelativeJump(Condition { flag, not }) => {
                        self.lsb = value;
                        if self.get_flag(flag) != not {
                            self.instruction_register = (
                                vec([Instruction::NoRead(Nop), Instruction::NoRead(OffsetPc)]),
                                SetPc(Register16Bit::WZ),
                            );
                        }
                    }
                    ConditionalCall(Condition { flag, not }) => {
                        self.msb = value; // msb not like jr
                        if self.get_flag(flag) != not {
                            self.instruction_register = (
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
                        self.msb = value; // msb not like jr
                        if self.get_flag(flag) != not {
                            self.instruction_register = (
                                vec([Nop.into(), Store16Bit(Register16Bit::PC).into()]),
                                Default::default(),
                            );
                        }
                    }
                }
            }
            NoRead(Nop) => {}
            NoRead(Store8Bit(register)) => {
                self.set_8bit_register(register, self.lsb);
            }
            NoRead(Store16Bit(register)) => {
                self.set_16bit_register(register, u16::from_be_bytes([self.msb, self.lsb]));
            }
            NoRead(Xor8Bit(register)) => {
                self.a ^= self.get_8bit_register(register);
                let flags = &mut self.f;
                flags.set(Flags::Z, self.a == 0);
                flags.remove(Flags::N);
                flags.remove(Flags::H);
                flags.remove(Flags::C);
            }
            NoRead(LoadToAddressHlFromADec) => {
                let hl = self.get_16bit_register(Register16Bit::HL);
                mmu.write(hl, self.a);
                [self.h, self.l] = hl.wrapping_sub(1).to_be_bytes();
            }
            NoRead(LoadToAddressHlFromAInc) => {
                let hl = self.get_16bit_register(Register16Bit::HL);
                mmu.write(hl, self.a);
                [self.h, self.l] = hl.wrapping_add(1).to_be_bytes();
            }
            NoRead(Bit8Bit(bit, register)) => {
                let result = (self.get_8bit_register(register) & (1 << bit)) == 0;
                let flags = &mut self.f;
                flags.set(Flags::Z, result);
                flags.remove(Flags::N);
                flags.insert(Flags::H);
            }
            NoRead(OffsetPc) => {
                self.set_16bit_register(
                    Register16Bit::WZ,
                    self.pc
                        .wrapping_add_signed(i16::from(self.lsb.cast_signed())),
                );
            }
            NoRead(LoadFromAccumulator(register)) => {
                mmu.write(
                    0xff00
                        | u16::from(
                            register
                                .map(|register| self.get_8bit_register(register))
                                .unwrap_or(self.lsb),
                        ),
                    self.a,
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
            NoRead(DecStackPointer) => self.sp = self.sp.wrapping_sub(1),
            NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(register)) => {
                mmu.write(self.sp, self.get_16bit_register(register).to_be_bytes()[0]);
                self.sp = self.sp.wrapping_sub(1);
            }
            NoRead(WriteLsbPcWhereSpPointsAndLoadCacheToPc) => {
                mmu.write(self.sp, self.pc.to_be_bytes()[1]);
                self.pc = u16::from_be_bytes([self.msb, self.lsb]);
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
                let value = self.a;
                let flags = &mut self.f;
                let carry = flags.contains(Flags::C);
                flags.set(Flags::C, (value & 0x1) == 0x1);
                let result = (value >> 1) | ((carry as u8) << 7);
                self.a = result;
                // difference with rr_r
                flags.remove(Flags::N | Flags::Z | Flags::H);
            }
            NoRead(Rla) => {
                let new_carry = (self.a & 0x80) == 0x80;
                self.a = (self.a << 1) | (self.f.contains(Flags::C) as u8);
                let flags = &mut self.f;
                // difference with rl r
                flags.remove(Flags::Z | Flags::N | Flags::H);
                flags.set(Flags::C, new_carry);
            }
            NoRead(Dec8Bit(register)) => {
                let r = self.get_8bit_register(register);
                let decremented = r.wrapping_sub(1);
                self.set_8bit_register(register, decremented);
                let flags = &mut self.f;
                flags.set(Flags::Z, decremented == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, set_h_sub(r, 1));
            }
            NoRead(DecHl) => {
                let r = self.lsb;
                let decremented = r.wrapping_sub(1);
                mmu.write(self.get_16bit_register(Register16Bit::HL), decremented);
                let flags = &mut self.f;
                flags.set(Flags::Z, decremented == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, set_h_sub(r, 1));
            }
            NoRead(LoadToCachedAddressFromA) => {
                mmu.write(u16::from_be_bytes([self.msb, self.lsb]), self.a);
            }
            NoRead(Sub8Bit(register)) => {
                let a = self.a;
                let r = self.get_8bit_register(register);
                let (result, carry) = a.overflowing_sub(r);
                let flags = &mut self.f;
                flags.set(Flags::Z, result == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, set_h_sub(a, r));
                flags.set(Flags::C, carry);
                self.a = result;
            }
            NoRead(Add8Bit(register)) => {
                let a = self.a;
                let register_value = self.get_8bit_register(register);
                let (result, carry) = a.overflowing_add(register_value);
                let flags = &mut self.f;
                flags.set(Flags::Z, result == 0);
                flags.remove(Flags::N);
                flags.set(Flags::H, set_h_add(a, register_value));
                flags.set(Flags::C, carry);
                self.a = result;
            }
            NoRead(Di) => self.ime = false,
            NoRead(Ei) => self.ime = true,
            NoRead(DecPc) => self.pc -= 1,
            NoRead(WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(address)) => {
                mmu.write(self.sp, self.pc.to_be_bytes()[1]);
                self.pc = u16::from(address);
            }
            NoRead(Res(bit, register)) => {
                self.set_8bit_register(register, self.get_8bit_register(register) & !(1 << bit));
            }
            NoRead(ResHl(bit)) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.lsb & !(1 << bit),
                );
            }
            NoRead(LoadToAddressHlN) => {
                mmu.write(self.get_16bit_register(Register16Bit::HL), self.lsb);
            }
            NoRead(Dec16Bit(register)) => {
                self.set_16bit_register(
                    register,
                    self.get_16bit_register(register).wrapping_sub(1),
                );
            }
            NoRead(Or8Bit(register)) => {
                let result = self.a | self.get_8bit_register(register);
                self.a = result;
                let flags = &mut self.f;
                flags.set(Flags::Z, result == 0);
                flags.remove(Flags::N | Flags::H | Flags::C);
            }
            NoRead(AddHlFirst(register)) => {
                let l = self.l;
                let register_value = self.get_8bit_register(register);
                let (result, carry) = l.overflowing_add(register_value);
                let flags = &mut self.f;
                // no z
                flags.remove(Flags::N);
                flags.set(Flags::H, set_h_add(l, register_value));
                flags.set(Flags::C, carry);
                self.l = result;
            }
            NoRead(AddHlSecond(register)) => {
                let h = self.h;
                let register_value = self.get_8bit_register(register);
                let (result, mut carry) = h.overflowing_add(register_value);
                let flags = &mut self.f;
                let (result, carry1) = result.overflowing_add(flags.contains(Flags::C) as u8);
                carry |= carry1;
                // no z
                flags.remove(Flags::N);
                flags.set(
                    Flags::H,
                    set_h_add_with_carry(h, register_value, flags.contains(Flags::C)),
                );
                flags.set(Flags::C, carry);
                self.h = result;
            }
            NoRead(ConditionalReturn(Condition { flag, not })) => {
                if self.get_flag(flag) != not {
                    self.instruction_register = (
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
                self.lsb | (1 << bit),
            ),
            NoRead(Halt) => self.is_halted = true,
            NoRead(Swap8Bit(register)) => {
                let result = self.swap(self.get_8bit_register(register));
                self.set_8bit_register(register, result);
            }
            NoRead(LoadHlFromAdjustedStackPointerFirst) => {
                let [_, sp_lsb] = self.sp.to_be_bytes();
                let (result, carry) = sp_lsb.overflowing_add(self.lsb);
                self.l = result;
                let flags = &mut self.f;
                flags.remove(Flags::Z | Flags::N);
                flags.set(Flags::H, set_h_add(sp_lsb, self.lsb));
                flags.set(Flags::C, carry);
            }
            NoRead(LoadHlFromAdjustedStackPointerSecond) => {
                [self.h, _] = self
                    .sp
                    .wrapping_add_signed(i16::from(self.lsb.cast_signed()))
                    .to_be_bytes()
            }
            NoRead(Cp8Bit(register)) => {
                let a = self.a;
                let value = self.get_8bit_register(register);
                let (result, carry) = a.overflowing_sub(value);
                let flags = &mut self.f;
                flags.set(Flags::Z, result == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, set_h_sub(a, value));
                flags.set(Flags::C, carry);
            }
            NoRead(LdSpHl) => self.sp = self.get_16bit_register(Register16Bit::HL),
            NoRead(Rlca) => {
                let a = self.a;
                let carry = a >> 7;
                self.a = (a << 1) | carry;
                let flags = &mut self.f;
                flags.remove(Flags::Z | Flags::N | Flags::H);
                flags.set(Flags::C, carry == 1);
            }
            NoRead(Stop) => {
                self.stop_mode = true;
                state.set_reset_system_clock(true);
            }
            NoRead(WriteLsbSpToCachedAddressAndIncCachedAddress) => {
                let [_, lsb] = self.sp.to_be_bytes();
                let wz = u16::from_be_bytes([self.msb, self.lsb]);
                mmu.write(wz, lsb);
                [self.msb, self.lsb] = wz.wrapping_add(1).to_be_bytes();
            }
            NoRead(WriteMsbSpToCachedAddress) => {
                let [msb, _] = self.sp.to_be_bytes();
                mmu.write(u16::from_be_bytes([self.msb, self.lsb]), msb);
            }
            NoRead(AddSpE) => {
                let [_, sp_lsb] = self.sp.to_be_bytes();
                let (_, carry) = sp_lsb.overflowing_add(self.lsb);
                let flags = &mut self.f;
                flags.remove(Flags::Z | Flags::N);
                flags.set(Flags::H, set_h_add(sp_lsb, self.lsb));
                flags.set(Flags::C, carry);
                self.sp = self
                    .sp
                    .wrapping_add_signed(i16::from(self.lsb.cast_signed()));
            }
            NoRead(Reti) => {
                self.pc = u16::from_be_bytes([self.msb, self.lsb]);
                self.ime = true;
            }
            NoRead(Cpl) => {
                self.f.insert(Flags::N | Flags::H);
                self.a = !self.a;
            }
            NoRead(Scf) => {
                let flags = &mut self.f;
                flags.remove(Flags::N | Flags::H);
                flags.insert(Flags::C);
            }
            NoRead(Ccf) => {
                let flags = &mut self.f;
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
                let result = self.a & self.get_8bit_register(register);
                self.a = result;
                let flags = &mut self.f;
                flags.set(Flags::Z, result == 0);
                flags.remove(Flags::N | Flags::C);
                flags.insert(Flags::H);
            }
            NoRead(Rrca) => {
                let a = self.a;
                self.a = a.rotate_right(1);
                let flags = &mut self.f;
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
                self.rlc(self.lsb),
            ),
            NoRead(RrcHl) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.rrc(self.lsb),
                );
            }
            NoRead(RlHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.rl(self.lsb),
            ),
            NoRead(RrHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.rr(self.lsb),
            ),
            NoRead(SlaHl) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.sla(self.lsb),
                );
            }
            NoRead(SraHl) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.sra(self.lsb),
                );
            }
            NoRead(SwapHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.swap(self.lsb),
            ),
            NoRead(SrlHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.srl(self.lsb),
            ),
            NoRead(IncHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.inc(self.lsb),
            ),
            NoRead(Daa) => {
                // https://rgbds.gbdev.io/docs/v1.0.0/gbz80.7#DAA
                let flags = &mut self.f;
                let mut adj = 0;
                let result = if flags.contains(Flags::N) {
                    if flags.contains(Flags::H) {
                        adj += 0x06
                    }
                    if flags.contains(Flags::C) {
                        adj += 0x60
                    }
                    self.a.wrapping_sub(adj)
                } else {
                    let a = self.a;
                    if flags.contains(Flags::H) || (a & 0x0f) > 0x09 {
                        adj += 0x06;
                    }
                    if flags.contains(Flags::C) || a > 0x99 {
                        adj += 0x60;
                        flags.insert(Flags::C);
                    }
                    self.a.wrapping_add(adj)
                };
                self.a = result;
                flags.set(Flags::Z, result == 0);
                flags.remove(Flags::H);
            }
            NoRead(CbMode) => self.is_cb_mode = true,
        }
    }

    fn adc(&mut self, second: u8) {
        let first = self.a as u32;
        let second = second as u32;
        let flags = &mut self.f;
        let carry = flags.contains(Flags::C) as u32;

        let result = first.wrapping_add(second).wrapping_add(carry);
        let result_b = result as u8;

        flags.remove(Flags::N);
        flags.set(Flags::Z, result_b == 0);
        flags.set(Flags::H, (first ^ second ^ result) & 0x10 == 0x10);
        flags.set(Flags::C, (result & 0x100) == 0x100);

        self.a = result_b;
    }

    fn sbc(&mut self, second: u8) {
        let first = self.a as u32;
        let second = second as u32;
        let flags = &mut self.f;
        let carry = flags.contains(Flags::C) as u32;

        let result = first.wrapping_sub(second).wrapping_sub(carry);
        let result_b = result as u8;

        flags.insert(Flags::N);
        flags.set(Flags::Z, result_b == 0);
        flags.set(Flags::H, (first ^ second ^ result) & 0x10 == 0x10);
        flags.set(Flags::C, (result & 0x100) == 0x100);

        self.a = result_b;
    }

    fn sla(&mut self, value: u8) -> u8 {
        let result = value << 1;
        let flags = &mut self.f;
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (value & 0x80) == 0x80);
        result
    }

    fn sra(&mut self, value: u8) -> u8 {
        let result = (value >> 1) | (value & 0x80);
        let flags = &mut self.f;
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (value & 0x1) == 0x1);
        result
    }

    fn inc(&mut self, value: u8) -> u8 {
        let incremented = value.wrapping_add(1);
        let flags = &mut self.f;
        flags.set(Flags::Z, incremented == 0);
        flags.remove(Flags::N);
        flags.set(Flags::H, set_h_add(value, 1));
        incremented
    }

    fn rrc(&mut self, value: u8) -> u8 {
        let flags = &mut self.f;
        flags.set(Flags::Z, value == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (value & 1) == 1);
        value.rotate_right(1)
    }

    fn rlc(&mut self, value: u8) -> u8 {
        let result = value.rotate_left(1);
        let flags = &mut self.f;
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (result & 1) == 1);
        result
    }

    fn rl(&mut self, value: u8) -> u8 {
        let new_carry = (value & 0x80) == 0x80;
        let result = (value << 1) | (self.f.contains(Flags::C) as u8);
        let flags = &mut self.f;
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, new_carry);
        result
    }

    fn srl(&mut self, value: u8) -> u8 {
        let result = value >> 1;
        let flags = &mut self.f;
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::C, (value & 0x1) == 0x1);
        result
    }

    fn rr(&mut self, value: u8) -> u8 {
        let flags = &mut self.f;
        let carry = flags.contains(Flags::C);
        flags.set(Flags::C, (value & 0x1) == 0x1);
        let result = (value >> 1) | ((carry as u8) << 7);
        flags.remove(Flags::N | Flags::H);
        flags.set(Flags::Z, result == 0);
        result
    }

    fn swap(&mut self, value: u8) -> u8 {
        let result = ((value >> 4) & 0x0f) | ((value << 4) & 0xf0);
        let flags = &mut self.f;
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
            // println!(
            //     "Interrupt handler: {interrupt:?}, enable: {:?}, lcd_status: {:?}",
            //     state.interrupt_enable, state.lcd_status
            // );
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
                // println!("Interrupt handling");
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
                let InstructionsAndSetPc((head, tail), set_pc) =
                    get_instructions(opcode, self.is_cb_mode);
                self.is_cb_mode = false;
                self.instruction_register.0 = tail;
                self.instruction_register.1 = set_pc;
                head
            }
        };

        let inst = match inst {
            Instruction::NoRead(no_read) => AfterReadInstruction::NoRead(no_read),
            Instruction::Read(ReadAddress::Accumulator, inst) => {
                AfterReadInstruction::Read(mmu.read(0xff00 | u16::from(self.lsb)), inst)
            }
            Instruction::Read(ReadAddress::Accumulator8Bit(register), inst) => {
                AfterReadInstruction::Read(
                    mmu.read(0xff00 | u16::from(self.get_8bit_register(register))),
                    inst,
                )
            }
            Instruction::Read(ReadAddress::Register { register, op }, inst) => {
                let register_value = self.get_16bit_register(register);
                match op {
                    OpAfterRead::None => {}
                    OpAfterRead::Inc => {
                        self.set_16bit_register(register, register_value.wrapping_add(1))
                    }
                    OpAfterRead::Dec => {
                        self.set_16bit_register(register, register_value.wrapping_sub(1))
                    }
                }
                AfterReadInstruction::Read(mmu.read(register_value), inst)
            }
        };

        Some(move |mut state: WriteOnlyState<'_>| {
            if let Some(flag) = interrupt_flag_to_reset {
                state.remove_if_bit(flag);
            }

            self.execute_instruction(state, inst);

            // https://gbdev.io/pandocs/halt.html#halt-bug
            if let AfterReadInstruction::NoRead(NoReadInstruction::Halt) = inst
                && !self.ime
                && !interrupts_to_execute.is_empty()
            {
                todo!("halt bug")
            }
        })
    }
}
