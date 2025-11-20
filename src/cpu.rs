use crate::{
    StateMachine,
    ic::Ints,
    instructions::{
        AfterReadInstruction, Condition, Flag, Instruction, Instructions, NoReadInstruction,
        ReadAddress, ReadInstruction, Register8Bit, Register16Bit, get_instructions, vec,
    },
    state::{MmuWrite, State, WriteOnlyState},
};
use arrayvec::ArrayVec;
use my_lib::HeapSize;

#[derive(HeapSize, Default)]
pub struct PipelineExecutor {
    sp: u16,
    lsb: u8,
    msb: u8,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    z_flag: bool,
    n_flag: bool,
    h_flag: bool,
    c_flag: bool,
    is_cb_mode: bool,
    pc: u16,
    instruction_register: Instructions,
    ime: bool,
}

enum PipelineAction {
    Pop,
    Replace(Instructions),
}

pub fn set_h_add(arg1: u8, arg2: u8) -> bool {
    let lo1 = arg1 & 0x0F;
    let lo2 = arg2 & 0x0F;

    ((lo1 + lo2) & (0x10)) == 0x10
}

pub fn set_h_sub(arg1: u8, arg2: u8) -> bool {
    let lo1 = arg1 & 0x0F;
    let lo2 = arg2 & 0x0F;

    (lo1.wrapping_sub(lo2) & (0x10)) == 0x10
}

impl PipelineExecutorWriteOnce<'_> {
    fn get_8bit_register(&self, register: Register8Bit) -> u8 {
        match register {
            Register8Bit::A => self.a.get(),
            Register8Bit::B => self.b.get(),
            Register8Bit::C => self.c.get(),
            Register8Bit::D => self.d.get(),
            Register8Bit::E => self.e.get(),
            Register8Bit::H => self.h.get(),
            Register8Bit::L => self.l.get(),
            Register8Bit::F => self.get_flag_bits(),
        }
    }

    fn get_8bit_register_mut(&mut self, register: Register8Bit) -> &mut u8 {
        match register {
            Register8Bit::A => self.a.get_mut(),
            Register8Bit::B => self.b.get_mut(),
            Register8Bit::C => self.c.get_mut(),
            Register8Bit::D => self.d.get_mut(),
            Register8Bit::E => self.e.get_mut(),
            Register8Bit::H => self.h.get_mut(),
            Register8Bit::L => self.l.get_mut(),
            Register8Bit::F => unreachable!(),
        }
    }

    fn get_16bit_register(&self, register: Register16Bit) -> u16 {
        match register {
            Register16Bit::AF => u16::from(self.a.get()) << 8 | u16::from(self.get_flag_bits()),
            Register16Bit::BC => u16::from(self.b.get()) << 8 | u16::from(self.c.get()),
            Register16Bit::DE => u16::from(self.d.get()) << 8 | u16::from(self.e.get()),
            Register16Bit::HL => u16::from(self.h.get()) << 8 | u16::from(self.l.get()),
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
        *self.get_8bit_register_mut(register.get_msb()) = msb;
        *self.get_8bit_register_mut(register.get_lsb()) = lsb;
    }

    fn get_flag_bits(&self) -> u8 {
        (self.z_flag.get() as u8) << 7
            | (self.n_flag.get() as u8) << 6
            | (self.h_flag.get() as u8) << 5
            | (self.c_flag.get() as u8) << 4
    }

    fn get_flag(&self, flag: Flag) -> bool {
        match flag {
            Flag::Z => self.z_flag.get(),
            Flag::N => self.n_flag.get(),
            Flag::H => self.h_flag.get(),
            Flag::C => self.c_flag.get(),
        }
    }

    fn execute_instruction(
        &mut self,
        mut mmu: MmuWrite,
        inst: AfterReadInstruction,
    ) -> PipelineAction {
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
                            return PipelineAction::Replace((
                                Instruction::NoRead(OffsetPc),
                                // important to match the cycle and not conflict with the overlapping opcode fetch
                                ArrayVec::from_iter([Instruction::NoRead(Nop)]),
                            ));
                        }
                    }
                    PopStackIntoLsb => {
                        *self.lsb.get_mut() = value;
                        *self.sp.get_mut() = self.sp.get().wrapping_add(1);
                    }
                    PopStackIntoMsb => {
                        *self.msb.get_mut() = value;
                        *self.sp.get_mut() = self.sp.get().wrapping_add(1);
                    }
                }
            }
            NoRead(Nop) => {}
            NoRead(Store8Bit(register)) => {
                *self.get_8bit_register_mut(register) = self.lsb.get();
            }
            NoRead(Store16Bit(register)) => {
                self.set_16bit_register(
                    register,
                    u16::from_be_bytes([self.msb.get(), self.lsb.get()]),
                );
            }
            NoRead(Xor(register)) => {
                *self.a.get_mut() ^= self.get_8bit_register(register);
                *self.z_flag.get_mut() = self.a.get() == 0;
                *self.n_flag.get_mut() = false;
                *self.h_flag.get_mut() = false;
                *self.c_flag.get_mut() = false;
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
            NoRead(Bit(bit, register)) => {
                *self.z_flag.get_mut() = (self.get_8bit_register(register) & (1 << bit)) == 0;
                *self.n_flag.get_mut() = false;
                *self.h_flag.get_mut() = true;
            }
            NoRead(OffsetPc) => {
                *self.pc.get_mut() =
                    (self.pc.get() as i16).wrapping_add(i16::from(self.lsb.get() as i8)) as u16;
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
                let r = self.get_8bit_register(register);
                let incremented = r.wrapping_add(1);
                *self.get_8bit_register_mut(register) = incremented;
                *self.z_flag.get_mut() = incremented == 0;
                *self.n_flag.get_mut() = false;
                *self.h_flag.get_mut() = set_h_add(r, 1);
            }
            NoRead(Inc16Bit(register)) => {
                self.set_16bit_register(
                    register,
                    self.get_16bit_register(register).wrapping_add(1),
                );
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
                *self.get_8bit_register_mut(to) = self.get_8bit_register(from);
            }
            NoRead(Rl(register)) => {
                let register_value = self.get_8bit_register(register);
                let new_carry = (register_value & 0x80) == 0x80;
                let new_value = (register_value << 1) | (self.c_flag.get() as u8);
                *self.get_8bit_register_mut(register) = new_value;
                *self.z_flag.get_mut() = new_value == 0;
                *self.n_flag.get_mut() = false;
                *self.h_flag.get_mut() = false;
                *self.c_flag.get_mut() = new_carry;
            }
            NoRead(Rla) => {
                let new_carry = (self.a.get() & 0x80) == 0x80;
                *self.a.get_mut() = (self.a.get() << 1) | (self.c_flag.get() as u8);
                *self.z_flag.get_mut() = false; // difference with rl r
                *self.n_flag.get_mut() = false;
                *self.h_flag.get_mut() = false;
                *self.c_flag.get_mut() = new_carry;
            }
            NoRead(Dec(register)) => {
                let r = self.get_8bit_register(register);
                let decremented = r.wrapping_sub(1);
                *self.get_8bit_register_mut(register) = decremented;
                *self.z_flag.get_mut() = decremented == 0;
                *self.n_flag.get_mut() = true;
                *self.h_flag.get_mut() = set_h_sub(r, 1);
            }
            NoRead(Compare) => {
                let a = self.a.get();
                let lsb = self.lsb.get();
                let (result, carry) = a.overflowing_sub(lsb);
                *self.z_flag.get_mut() = result == 0;
                *self.n_flag.get_mut() = true;
                *self.h_flag.get_mut() = set_h_sub(a, lsb);
                *self.c_flag.get_mut() = carry;
            }
            NoRead(LoadToCachedAddressFromA) => {
                mmu.write(
                    u16::from_be_bytes([self.msb.get(), self.lsb.get()]),
                    self.a.get(),
                );
            }
            NoRead(Sub(register)) => {
                let a = self.a.get();
                let r = self.get_8bit_register(register);
                let (result, carry) = a.overflowing_sub(r);
                *self.z_flag.get_mut() = result == 0;
                *self.n_flag.get_mut() = true;
                *self.h_flag.get_mut() = set_h_sub(a, r);
                *self.c_flag.get_mut() = carry;
                *self.a.get_mut() = result;
            }
            NoRead(Add) => {
                let a = self.a.get();
                let lsb = self.lsb.get();
                let (result, carry) = a.overflowing_add(lsb);
                *self.z_flag.get_mut() = result == 0;
                *self.n_flag.get_mut() = false;
                *self.h_flag.get_mut() = set_h_add(a, lsb);
                *self.c_flag.get_mut() = carry;
                *self.a.get_mut() = result;
            }
            NoRead(Add8Bit(register)) => {
                let a = self.a.get();
                let register_value = self.get_8bit_register(register);
                let (result, carry) = a.overflowing_add(register_value);
                *self.z_flag.get_mut() = result == 0;
                *self.n_flag.get_mut() = false;
                *self.h_flag.get_mut() = set_h_add(a, register_value);
                *self.c_flag.get_mut() = carry;
                *self.a.get_mut() = result;
            }
            NoRead(Di) => {
                *self.ime.get_mut() = false;
            }
            NoRead(DecPc) => {
                *self.pc.get_mut() -= 1;
            }
            NoRead(WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(address)) => {
                mmu.write(self.sp.get(), self.pc.get().to_be_bytes()[1]);
                *self.pc.get_mut() = address;
            }
            NoRead(Res(bit, register)) => {
                *self.get_8bit_register_mut(register) &= !(1 << bit);
            }
            NoRead(And) => {
                let result = self.a.get() & self.lsb.get();
                *self.a.get_mut() = result;
                *self.z_flag.get_mut() = result == 0;
                *self.n_flag.get_mut() = false;
                *self.h_flag.get_mut() = true;
                *self.c_flag.get_mut() = false;
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
        }

        PipelineAction::Pop
    }

    pub fn pipeline_pop_front(&mut self) {
        // does nothing if there is only one instruction inside the pipeline
        // if there is only one instruction then the OpcodeFetcher will override the whole pipeline
        if self.instruction_register.get_ref().1.is_empty() {
            // we can't use pop because the WriteOnce will panic
            return;
        }
        let instruction_register = self.instruction_register.get_mut();
        instruction_register.0 = instruction_register.1.pop().unwrap();
    }
}

impl StateMachine for PipelineExecutor {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        // we load the next opcode if there is only one instruction left in the pipeline
        let mmu = state.mmu();

        let interrupts_to_execute = state.interrupt_enable & state.interrupt_flag;

        let mut interrupt_flag_to_reset = Option::<Ints>::None;

        let mut write_once = self.write_once();

        // https://gbdev.io/pandocs/Interrupt_Sources.html
        if write_once.ime.get()
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
            interrupt_flag_to_reset = Some(interrupt);
            *write_once.ime.get_mut() = false;
            // https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#isr-and-nmi
            use NoReadInstruction::*;
            *write_once.instruction_register.get_mut() = (
                DecPc.into(),
                vec([
                    Nop.into(),
                    WriteLsbPcWhereSpPointsAndLoadAbsoluteAddressToPc(address).into(),
                    WriteMsbOfRegisterWhereSpPointsAndDecSp(Register16Bit::PC).into(),
                    DecStackPointer.into(),
                ]),
            );
        }

        let should_load_next_opcode = write_once.instruction_register.get_ref().1.is_empty();

        let opcode = mmu.read(write_once.pc.get());

        // if should_load_next_opcode {
        //     println!(
        //         "Read opcode at ${:04x} (0x{opcode:02x})",
        //         write_once.pc.get()
        //     );
        // }

        let inst = write_once.instruction_register.get_ref().0;

        let should_increment_pc = matches!(
            inst,
            Instruction::Read(ReadAddress::Register(Register16Bit::PC), _)
        );

        // print!("Executing {inst:?}");

        let inst = match inst {
            Instruction::NoRead(no_read) => AfterReadInstruction::NoRead(no_read),
            Instruction::Read(ReadAddress::Accumulator, read) => {
                AfterReadInstruction::Read(mmu.read(0xff00 | (write_once.lsb.get() as u16)), read)
            }
            Instruction::Read(ReadAddress::Register(register), read) => {
                AfterReadInstruction::Read(mmu.read(write_once.get_16bit_register(register)), read)
            }
        };

        // if let AfterReadInstruction::Read(value, _) = inst {
        //     print!(", read: 0x{value:x}");
        // }

        // println!();

        move |mut state| {
            if let Some(flag) = interrupt_flag_to_reset {
                state.remove_if_bit(flag);
            }
            if should_increment_pc {
                *write_once.pc.get_mut() = write_once.pc.get().wrapping_add(1);
            }

            match write_once.execute_instruction(state.mmu(), inst) {
                PipelineAction::Pop => write_once.pipeline_pop_front(),
                PipelineAction::Replace(instructions) => {
                    *write_once.instruction_register.get_mut() = instructions
                }
            }

            if should_load_next_opcode {
                *write_once.instruction_register.get_mut() =
                    get_instructions(opcode, write_once.is_cb_mode.get());
                *write_once.is_cb_mode.get_mut() = opcode == 0xcb;
                *write_once.pc.get_mut() = write_once.pc.get().wrapping_add(1);
            }
        }
    }
}
