use crate::{
    StateMachine,
    ic::Ints,
    instructions::{
        AfterReadInstruction, Condition, Flag, Instruction, InstructionsAndSetPc,
        NoReadInstruction, OpAfterRead, POP_SP, Prefetch, ReadAddress, ReadInstruction,
        Register8Bit, Register16Bit, SetPc, get_instructions, vec,
    },
    state::{MmuReadCpu, MmuWrite, State},
};

use arrayvec::ArrayVec;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Ime {
    Off,
    On,
    // https://gbdev.io/pandocs/Interrupts.html#ime-interrupt-master-enable-flag-write-only
    Delay,
}

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
    pub instruction_register: (ArrayVec<Instruction, 5>, Prefetch),
    pub ime: bool,
    pub is_halted: bool,
    pub stop_mode: bool,
    // test purposes
    pub current_opcode: u8,
    pub is_dispatching_interrupt: bool,
    pub interrupt_flag: Ints,
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
            ime: false,
            is_halted: Default::default(),
            stop_mode: Default::default(),
            current_opcode: 0,
            is_dispatching_interrupt: false,
            interrupt_flag: Ints::empty(),
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

// Comment ça se passe avec mooneye
// le cpu drive l'ensemble
// pour une lecture d'un registre, il fait d'abord un cycle chez les périphériques, et ensuite il lit la valeur.
// Donc quand le cycle d'un périphérique donne une interruption, cela n'affecte pas
// le cpu dans le cycle actuel (puisqu'il est en train de faire l'action de lecture).
// Donc il faut traiter l'interruption dans le prochain cycle.
// Pour l'instant, il semble que les écritures/lectures du CPU sont toujours traités à la fin d'un cycle.
// Par exemple, il écrase les modif du timer pendant le cycle courant, et il a conscience des changements immédiats du ppu

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

    fn enable_ime(&mut self) {
        self.ime = true;
    }

    fn execute_instruction(
        &mut self,
        mut mmu: MmuWrite,
        inst: AfterReadInstruction,
        interrupts_to_execute: Ints,
        cycle_count: u64,
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
                        } else {
                            log::warn!("Conditional jump fail");
                        }
                    }
                }
            }
            NoRead(Nop) => {}
            NoRead(Store8Bit(register)) => {
                // if register == Register8Bit::A {
                //     log::warn!("Setting A to 0x{:02x}", self.lsb);
                // }
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
                mmu.write(hl, self.a, cycle_count, &mut self.interrupt_flag);
                [self.h, self.l] = hl.wrapping_sub(1).to_be_bytes();
            }
            NoRead(LoadToAddressHlFromAInc) => {
                let hl = self.get_16bit_register(Register16Bit::HL);
                mmu.write(hl, self.a, cycle_count, &mut self.interrupt_flag);
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
                    cycle_count,
                    &mut self.interrupt_flag,
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
                    cycle_count,
                    &mut self.interrupt_flag,
                );
            }
            NoRead(DecStackPointer) => self.sp = self.sp.wrapping_sub(1),
            NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(register)) => {
                mmu.write(
                    self.sp,
                    self.get_16bit_register(register).to_be_bytes()[0],
                    cycle_count,
                    &mut self.interrupt_flag,
                );
                self.sp = self.sp.wrapping_sub(1);
            }
            NoRead(WriteLsbPcWhereSpPointsAndLoadCacheToPc) => {
                mmu.write(
                    self.sp,
                    self.pc.to_be_bytes()[1],
                    cycle_count,
                    &mut self.interrupt_flag,
                );
                // log::warn!(
                //     "Set PC to ${:04x}",
                //     u16::from_be_bytes([self.msb, self.lsb])
                // );
                self.pc = u16::from_be_bytes([self.msb, self.lsb]);
                if self.pc == 0x0416 {
                    log::warn!("PC is at setup_and_wait");
                }
                if self.pc == 0x03fe {
                    log::warn!("PC is at standard_delay");
                }
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
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    decremented,
                    cycle_count,
                    &mut self.interrupt_flag,
                );
                let flags = &mut self.f;
                flags.set(Flags::Z, decremented == 0);
                flags.insert(Flags::N);
                flags.set(Flags::H, set_h_sub(r, 1));
            }
            NoRead(LoadToCachedAddressFromA) => {
                mmu.write(
                    u16::from_be_bytes([self.msb, self.lsb]),
                    self.a,
                    cycle_count,
                    &mut self.interrupt_flag,
                );
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
            NoRead(Di) => {
                log::warn!("{cycle_count}: Executing DI");
                self.ime = false
            }
            NoRead(Ei) => self.enable_ime(),
            NoRead(DecPc) => {
                log::warn!("{cycle_count}: Dec PC");
                self.pc -= 1
            }
            NoRead(WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(address)) => {
                mmu.write(
                    self.sp,
                    self.pc.to_be_bytes()[1],
                    cycle_count,
                    &mut self.interrupt_flag,
                );
                self.pc = u16::from(address);
            }
            NoRead(FinalStepInterruptDispatch) => {
                mmu.write(
                    self.sp,
                    self.pc.to_be_bytes()[1],
                    cycle_count,
                    &mut self.interrupt_flag,
                );
                // thanks https://github.com/Gekkio/mooneye-gb/blob/3856dcbca82a7d32bd438cc92fd9693f868e2e23/core/src/cpu.rs#L139
                let interrupt = interrupts_to_execute.iter().next();
                // we have to check here the interrupts to pass the ie_push test
                self.pc = match interrupt {
                    Some(Ints::VBLANK) => 0x0040,
                    Some(Ints::LCD) => 0x0048,
                    Some(Ints::TIMER) => 0x0050,
                    Some(Ints::SERIAL) => 0x0058,
                    Some(Ints::JOYPAD) => 0x0060,
                    _ => 0x0000,
                };
                if let Some(interrupt) = interrupt {
                    self.interrupt_flag.remove(interrupt);
                }
            }
            NoRead(Res(bit, register)) => {
                self.set_8bit_register(register, self.get_8bit_register(register) & !(1 << bit));
            }
            NoRead(ResHl(bit)) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.lsb & !(1 << bit),
                    cycle_count,
                    &mut self.interrupt_flag,
                );
            }
            NoRead(LoadToAddressHlN) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.lsb,
                    cycle_count,
                    &mut self.interrupt_flag,
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
                cycle_count,
                &mut self.interrupt_flag,
            ),
            // doesn't halt if there are interrupts https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#halt
            NoRead(Halt) => {
                log::warn!(
                    "{cycle_count}: HALT {} {:?}",
                    interrupts_to_execute.is_empty(),
                    self.ime
                );
                self.is_halted = interrupts_to_execute.is_empty();
            }
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
                if register == Register8Bit::E || register == Register8Bit::D && self.pc <= 0x0187 {
                    log::warn!(
                        "{cycle_count}: CP {register:?} (0x{:02x}) and A (0x{:02x}) with scx",
                        value,
                        self.a
                    );
                }
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
                todo!("reset sys clock");
            }
            NoRead(WriteLsbSpToCachedAddressAndIncCachedAddress) => {
                let [_, lsb] = self.sp.to_be_bytes();
                let wz = u16::from_be_bytes([self.msb, self.lsb]);
                mmu.write(wz, lsb, cycle_count, &mut self.interrupt_flag);
                [self.msb, self.lsb] = wz.wrapping_add(1).to_be_bytes();
            }
            NoRead(WriteMsbSpToCachedAddress) => {
                let [msb, _] = self.sp.to_be_bytes();
                mmu.write(
                    u16::from_be_bytes([self.msb, self.lsb]),
                    msb,
                    cycle_count,
                    &mut self.interrupt_flag,
                );
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
            NoRead(RlcHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.rlc(self.lsb),
                cycle_count,
                &mut self.interrupt_flag,
            ),
            NoRead(RrcHl) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.rrc(self.lsb),
                    cycle_count,
                    &mut self.interrupt_flag,
                );
            }
            NoRead(RlHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.rl(self.lsb),
                cycle_count,
                &mut self.interrupt_flag,
            ),
            NoRead(RrHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.rr(self.lsb),
                cycle_count,
                &mut self.interrupt_flag,
            ),
            NoRead(SlaHl) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.sla(self.lsb),
                    cycle_count,
                    &mut self.interrupt_flag,
                );
            }
            NoRead(SraHl) => {
                mmu.write(
                    self.get_16bit_register(Register16Bit::HL),
                    self.sra(self.lsb),
                    cycle_count,
                    &mut self.interrupt_flag,
                );
            }
            NoRead(SwapHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.swap(self.lsb),
                cycle_count,
                &mut self.interrupt_flag,
            ),
            NoRead(SrlHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.srl(self.lsb),
                cycle_count,
                &mut self.interrupt_flag,
            ),
            NoRead(IncHl) => mmu.write(
                self.get_16bit_register(Register16Bit::HL),
                self.inc(self.lsb),
                cycle_count,
                &mut self.interrupt_flag,
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
    fn execute(&mut self, state: &mut State, cycle_count: u64) {
        let interrupts_to_execute =
            Ints::from_bits_truncate(state.interrupt_enable.bits()) & self.interrupt_flag;

        // https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#nop-and-stop
        if self.stop_mode {
            // self.stop_mode = false;
            // // quand on va sortir du stop mode on va exécuter un nop
            // // et fetch le prochain opcode en parallèle
            // self.instruction_register = (vec([NoReadInstruction::Nop.into()]), Default::default());
            todo!("stop")
        }

        // https://gbdev.io/pandocs/halt.html#halt
        if self.is_halted {
            if interrupts_to_execute.is_empty() {
                self.interrupt_flag |= state.interrupt_flag;
                state.interrupt_flag = Ints::empty();
                return;
            }
            self.is_halted = false;
            self.instruction_register = (vec([NoReadInstruction::Nop.into()]), Default::default());
            log::warn!("{cycle_count}: Exiting HALT mode");
        }

        let mmu = MmuReadCpu(state.mmu());

        let inst = if let Some(inst) = self.instruction_register.0.pop() {
            inst
        } else if self.is_dispatching_interrupt {
            self.ime = false;
            // no need to set is_dispatching_interrupt to false
            use NoReadInstruction::*;
            log::warn!(
                "{cycle_count}: Interrupt handling ${interrupts_to_execute:?} ${:?}",
                state.lcd_status
            );
            self.instruction_register.0 = vec([
                Nop.into(),
                FinalStepInterruptDispatch.into(),
                WriteMsbOfRegisterWhereSpPointsAndDecSp(Register16Bit::PC).into(),
                DecStackPointer.into(),
            ]);
            self.instruction_register.1 = Default::default();
            DecPc.into()
        } else {
            let InstructionsAndSetPc((head, tail), set_pc) =
                get_instructions(self.current_opcode, self.is_cb_mode);
            self.is_cb_mode = false;
            self.instruction_register.0 = tail;
            self.instruction_register.1 = set_pc;
            head
        };

        // if cycle_count > 5981848 && cycle_count < 5999410 {
        //     log::warn!("{cycle_count}: Will execute {inst:?}");

        // }

        // fetch step
        if self.instruction_register.0.is_empty() {
            self.is_dispatching_interrupt = self.ime
                && self.instruction_register.1.check_interrupts
                && !interrupts_to_execute.is_empty();
            (self.pc, self.current_opcode) = match self.instruction_register.1.set_pc {
                SetPc::WithIncrement(register) => {
                    let address = self.get_16bit_register(register);
                    let opcode = mmu.read(address, cycle_count, self.interrupt_flag);
                    // if address == 0x4879 {
                    //     panic!("fail")
                    // }
                    // log::warn!("${address:04x} => ${opcode:2x}");
                    if opcode == 0x04 {
                        log::warn!("${address:04x} => INC B");
                    }
                    if opcode == 0x77 {
                        log::warn!("${address:04x} => LD (HL), A");
                    }
                    if opcode == 0xff {
                        log::warn!("${address:04x} => RST 0x38");
                    }
                    if opcode == 0xf3 {
                        log::warn!("{cycle_count}: ${address:04x} => DI");
                    }
                    // if opcode == 0x00 {
                    //     log::warn!("{cycle_count}: ${address:04x} => NOP");
                    // }
                    if opcode == 0xfb {
                        log::warn!("{cycle_count}: ${address:04x} => EI");
                    }

                    (address.wrapping_add(1), opcode)
                }
                SetPc::NoIncrement => {
                    (self.pc, mmu.read(self.pc, cycle_count, self.interrupt_flag))
                }
            };
        }

        // todo revoir la logique de lecture
        let inst = match inst {
            Instruction::NoRead(no_read) => AfterReadInstruction::NoRead(no_read),
            Instruction::Read(ReadAddress::Accumulator, inst) => AfterReadInstruction::Read(
                mmu.read(
                    0xff00 | u16::from(self.lsb),
                    cycle_count,
                    self.interrupt_flag,
                ),
                inst,
            ),
            Instruction::Read(ReadAddress::Accumulator8Bit(register), inst) => {
                AfterReadInstruction::Read(
                    mmu.read(
                        0xff00 | u16::from(self.get_8bit_register(register)),
                        cycle_count,
                        self.interrupt_flag,
                    ),
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
                AfterReadInstruction::Read(
                    mmu.read(register_value, cycle_count, self.interrupt_flag),
                    inst,
                )
            }
        };

        self.execute_instruction(MmuWrite(state), inst, interrupts_to_execute, cycle_count);

        self.interrupt_flag |= state.interrupt_flag;
        state.interrupt_flag = Ints::empty();
    }
}
