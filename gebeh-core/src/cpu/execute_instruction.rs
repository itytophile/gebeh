use super::instructions::{
    AfterReadInstruction, Condition, Flag, Instruction, NoReadInstruction, POP_SP, Prefetch,
    ReadInstruction, Register16Bit, SetPc, vec,
};
use crate::cpu::{Cpu, Flags};
use crate::wram::DmgWram;
use crate::{Peripherals, interrupts::Interrupts, mbc::Mbc};

fn is_half_carry(a: u8, b: u8, result: u8) -> bool {
    (a ^ b ^ result) & 0x10 != 0
}

impl Cpu {
    fn adc(&mut self, x: u8) {
        let sum = u16::from(self.a) + u16::from(x) + self.f.contains(Flags::C) as u16;

        self.f.remove(Flags::N);
        self.f.set(Flags::Z, sum as u8 == 0);
        self.f.set(Flags::H, is_half_carry(self.a, x, sum as u8));
        self.f.set(Flags::C, sum > 0xff);

        self.a = sum as u8;
    }

    fn sbc(&mut self, x: u8) {
        let carry = self.f.contains(Flags::C) as u8;
        let res = self.a.wrapping_sub(x).wrapping_sub(carry);

        self.f.insert(Flags::N);
        self.f.set(Flags::Z, res == 0);
        self.f.set(Flags::H, is_half_carry(self.a, x, res));
        self.f.set(
            Flags::C,
            u16::from(self.a) < (u16::from(x) + u16::from(carry)),
        );

        self.a = res;
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
        flags.set(Flags::H, is_half_carry(value, 1, incremented));
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

    fn get_flag(&self, flag: Flag) -> bool {
        match flag {
            Flag::Z => self.f.contains(Flags::Z),
            Flag::C => self.f.contains(Flags::C),
        }
    }

    fn enable_ime(&mut self) {
        self.ime = true;
    }

    pub(super) fn execute_instruction<M: Mbc + ?Sized>(
        &mut self,
        inst: AfterReadInstruction,
        interrupts_to_execute: Interrupts,
        cycle_count: u64,
        peripherals: &mut Peripherals<M, DmgWram>,
    ) {
        use AfterReadInstruction::*;
        use NoReadInstruction::*;
        use ReadInstruction::*;

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
                                Prefetch {
                                    check_interrupts: true,
                                    set_pc: SetPc::WithIncrement(Register16Bit::WZ),
                                },
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
                            // panic!("Conditional jump success");
                            self.instruction_register = (
                                vec([Nop.into(), Store16Bit(Register16Bit::PC).into()]),
                                Default::default(),
                            );
                        }
                    }
                }
            }
            NoRead(Nop) => {}
            NoRead(Store8Bit(register)) => self.set_8bit_register(register, self.lsb),
            NoRead(Store16Bit(register)) => {
                self.set_16bit_register(register, u16::from_be_bytes([self.msb, self.lsb]))
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
                self.write(hl, self.a, peripherals, cycle_count);
                [self.h, self.l] = hl.wrapping_sub(1).to_be_bytes();
            }
            NoRead(LoadToAddressHlFromAInc) => {
                let hl = self.get_16bit_register(Register16Bit::HL);
                self.write(hl, self.a, peripherals, cycle_count);
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
                self.write(
                    0xff00
                        | u16::from(
                            register
                                .map(|register| self.get_8bit_register(register))
                                .unwrap_or(self.lsb),
                        ),
                    self.a,
                    peripherals,
                    cycle_count,
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
                self.write(
                    self.get_16bit_register(address),
                    self.get_8bit_register(value),
                    peripherals,
                    cycle_count,
                );
            }
            NoRead(DecStackPointer) => self.sp = self.sp.wrapping_sub(1),
            NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(register)) => {
                self.write(
                    self.sp,
                    self.get_16bit_register(register).to_be_bytes()[0],
                    peripherals,
                    cycle_count,
                );
                self.sp = self.sp.wrapping_sub(1);
            }
            NoRead(WriteLsbPcWhereSpPointsAndLoadCacheToPc) => {
                self.write(self.sp, self.pc.to_be_bytes()[1], peripherals, cycle_count);
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
                flags.set(Flags::H, is_half_carry(r, 1, decremented));
            }
            NoRead(DecHl) => {
                let r = self.lsb;
                let decremented = r.wrapping_sub(1);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    decremented,
                    peripherals,
                    cycle_count,
                );
                let flags = &mut self.f;
                flags.set(Flags::Z, decremented == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, is_half_carry(r, 1, decremented));
            }
            NoRead(LoadToCachedAddressFromA) => {
                self.write(
                    u16::from_be_bytes([self.msb, self.lsb]),
                    self.a,
                    peripherals,
                    cycle_count,
                );
            }
            NoRead(Sub8Bit(register)) => {
                let a = self.a;
                let r = self.get_8bit_register(register);
                let (result, carry) = a.overflowing_sub(r);
                let flags = &mut self.f;
                flags.set(Flags::Z, result == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, is_half_carry(a, r, result));
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
                flags.set(Flags::H, is_half_carry(a, register_value, result));
                flags.set(Flags::C, carry);
                self.a = result;
            }
            NoRead(Di) => self.ime = false,
            NoRead(Ei) => self.enable_ime(),
            NoRead(DecPc) => self.pc -= 1,
            NoRead(WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(address)) => {
                self.write(self.sp, self.pc.to_be_bytes()[1], peripherals, cycle_count);
                self.pc = u16::from(address);
            }
            NoRead(FinalStepInterruptDispatch) => {
                self.write(self.sp, self.pc.to_be_bytes()[1], peripherals, cycle_count);
                // thanks https://github.com/Gekkio/mooneye-gb/blob/3856dcbca82a7d32bd438cc92fd9693f868e2e23/core/src/cpu.rs#L139
                let interrupt = interrupts_to_execute.iter().next();
                // we have to check here the interrupts to pass the ie_push test
                self.pc = match interrupt {
                    Some(Interrupts::VBLANK) => 0x0040,
                    Some(Interrupts::LCD) => 0x0048,
                    Some(Interrupts::TIMER) => 0x0050,
                    Some(Interrupts::SERIAL) => 0x0058,
                    Some(Interrupts::JOYPAD) => 0x0060,
                    _ => 0x0000,
                };
                if let Some(interrupt) = interrupt {
                    peripherals.interrupts.remove(interrupt);
                }
            }
            NoRead(Res(bit, register)) => {
                self.set_8bit_register(register, self.get_8bit_register(register) & !(1 << bit));
            }
            NoRead(ResHl(bit)) => {
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.lsb & !(1 << bit),
                    peripherals,
                    cycle_count,
                );
            }
            NoRead(LoadToAddressHlN) => {
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.lsb,
                    peripherals,
                    cycle_count,
                );
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
                flags.set(Flags::H, is_half_carry(l, register_value, result));
                flags.set(Flags::C, carry);
                self.l = result;
            }
            NoRead(AddHlSecond(register)) => {
                let r = self.get_8bit_register(register);
                let sum = u16::from(self.h) + u16::from(r) + self.f.contains(Flags::C) as u16;

                // no z
                self.f.remove(Flags::N);
                self.f.set(Flags::H, is_half_carry(self.h, r, sum as u8));
                self.f.set(Flags::C, sum > 0xff);
                self.h = sum as u8;
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
            NoRead(SetHl(bit)) => self.write(
                self.get_16bit_register(Register16Bit::HL),
                self.lsb | (1 << bit),
                peripherals,
                cycle_count,
            ),
            // doesn't halt if there are interrupts https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#halt
            NoRead(Halt) => self.is_halted = interrupts_to_execute.is_empty(),
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
                flags.set(Flags::H, is_half_carry(sp_lsb, self.lsb, result));
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
                flags.set(Flags::H, is_half_carry(a, value, result));
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
                todo!("reset sys clock");
            }
            NoRead(WriteLsbSpToCachedAddressAndIncCachedAddress) => {
                let [_, lsb] = self.sp.to_be_bytes();
                let wz = u16::from_be_bytes([self.msb, self.lsb]);
                self.write(wz, lsb, peripherals, cycle_count);
                [self.msb, self.lsb] = wz.wrapping_add(1).to_be_bytes();
            }
            NoRead(WriteMsbSpToCachedAddress) => {
                let [msb, _] = self.sp.to_be_bytes();
                self.write(
                    u16::from_be_bytes([self.msb, self.lsb]),
                    msb,
                    peripherals,
                    cycle_count,
                );
            }
            NoRead(AddSpE) => {
                let [_, sp_lsb] = self.sp.to_be_bytes();
                let (result, carry) = sp_lsb.overflowing_add(self.lsb);
                let flags = &mut self.f;
                flags.remove(Flags::Z | Flags::N);
                flags.set(Flags::H, is_half_carry(sp_lsb, self.lsb, result));
                flags.set(Flags::C, carry);
                self.sp = self
                    .sp
                    .wrapping_add_signed(i16::from(self.lsb.cast_signed()));
            }
            NoRead(Reti) => {
                self.pc = u16::from_be_bytes([self.msb, self.lsb]);
                self.enable_ime();
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
            NoRead(RlcHl) => {
                let value = self.rlc(self.lsb);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    value,
                    peripherals,
                    cycle_count,
                )
            }
            NoRead(RrcHl) => {
                let value = self.rrc(self.lsb);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    value,
                    peripherals,
                    cycle_count,
                );
            }
            NoRead(RlHl) => {
                let value = self.rl(self.lsb);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    value,
                    peripherals,
                    cycle_count,
                )
            }
            NoRead(RrHl) => {
                let value = self.rr(self.lsb);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    value,
                    peripherals,
                    cycle_count,
                )
            }
            NoRead(SlaHl) => {
                let value = self.sla(self.lsb);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    value,
                    peripherals,
                    cycle_count,
                );
            }
            NoRead(SraHl) => {
                let value = self.sra(self.lsb);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    value,
                    peripherals,
                    cycle_count,
                );
            }
            NoRead(SwapHl) => {
                let value = self.swap(self.lsb);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    value,
                    peripherals,
                    cycle_count,
                )
            }
            NoRead(SrlHl) => {
                let value = self.srl(self.lsb);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    value,
                    peripherals,
                    cycle_count,
                )
            }
            NoRead(IncHl) => {
                let value = self.inc(self.lsb);
                self.write(
                    self.get_16bit_register(Register16Bit::HL),
                    value,
                    peripherals,
                    cycle_count,
                )
            }
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
}
