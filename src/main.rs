use arrayvec::ArrayVec;
use my_lib::HeapSize;

use crate::state::{
    AfterReadInstruction, Condition, Flag, Instruction, Instructions, NoReadInstruction,
    ReadAddress, ReadInstruction, Register8Bit, Register16Bit, State, WriteOnlyState,
    get_instructions,
};
mod state;

pub const DMG_BOOT: [u8; 256] = [
    49, 254, 255, 33, 255, 159, 175, 50, 203, 124, 32, 250, 14, 17, 33, 38, 255, 62, 128, 50, 226,
    12, 62, 243, 50, 226, 12, 62, 119, 50, 226, 17, 4, 1, 33, 16, 128, 26, 205, 184, 0, 26, 203,
    55, 205, 184, 0, 19, 123, 254, 52, 32, 240, 17, 204, 0, 6, 8, 26, 19, 34, 35, 5, 32, 249, 33,
    4, 153, 1, 12, 1, 205, 177, 0, 62, 25, 119, 33, 36, 153, 14, 12, 205, 177, 0, 62, 145, 224, 64,
    6, 16, 17, 212, 0, 120, 224, 67, 5, 123, 254, 216, 40, 4, 26, 224, 71, 19, 14, 28, 205, 167, 0,
    175, 144, 224, 67, 5, 14, 28, 205, 167, 0, 175, 176, 32, 224, 224, 67, 62, 131, 205, 159, 0,
    14, 39, 205, 167, 0, 62, 193, 205, 159, 0, 17, 138, 1, 240, 68, 254, 144, 32, 250, 27, 122,
    179, 32, 245, 24, 73, 14, 19, 226, 12, 62, 135, 226, 201, 240, 68, 254, 144, 32, 250, 13, 32,
    247, 201, 120, 34, 4, 13, 32, 250, 201, 71, 14, 4, 175, 197, 203, 16, 23, 193, 203, 16, 23, 13,
    32, 245, 34, 35, 34, 35, 201, 60, 66, 185, 165, 185, 165, 66, 60, 0, 84, 168, 252, 66, 79, 79,
    84, 73, 88, 46, 68, 77, 71, 32, 118, 49, 46, 50, 0, 62, 255, 198, 1, 11, 30, 216, 33, 77, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 1, 224, 80,
];

fn main() {
    // let rom =
    //     std::fs::read("/home/ityt/Téléchargements/pocket/pocket.gb")
    //         .unwrap();

    let mut state = State::default();
    state.memory[0..DMG_BOOT.len()].copy_from_slice(&DMG_BOOT);
    // the machine should not be affected by the composition order
    let mut machine = PipelineExecutor::default();

    loop {
        machine.execute(&state)(WriteOnlyState::new(&mut state));
    }
}

trait StateMachine {
    /// must take one M-cycle
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a;
    fn compose<T: StateMachine>(self, other: T) -> (Self, T)
    where
        Self: Sized,
    {
        (self, other)
    }
}

#[derive(HeapSize, Default)]
struct PipelineExecutor {
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
}

enum PipelineAction {
    Pop,
    Replace(Instructions),
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
        mut state: WriteOnlyState,
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
                state.write(hl, self.a.get());
                [*self.h.get_mut(), *self.l.get_mut()] = hl.wrapping_sub(1).to_be_bytes();
            }
            NoRead(LoadToAddressHlFromAInc) => {
                let hl = self.get_16bit_register(Register16Bit::HL);
                state.write(hl, self.a.get());
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
                state.write(
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
                let register_value = self.get_8bit_register(register);
                let incremented = register_value.wrapping_add(1);
                *self.get_8bit_register_mut(register) = incremented;
                *self.z_flag.get_mut() = incremented == 0;
                *self.n_flag.get_mut() = false;
                *self.h_flag.get_mut() = (register_value & 0x0F) == 0x0F;
            }
            NoRead(Inc16Bit(register)) => {
                self.set_16bit_register(
                    register,
                    self.get_16bit_register(register).wrapping_sub(1),
                );
            }
            NoRead(LoadToAddressFromRegister { address, value }) => {
                state.write(
                    self.get_16bit_register(address),
                    self.get_8bit_register(value),
                );
            }
            NoRead(DecStackPointer) => {
                *self.sp.get_mut() = self.sp.get().wrapping_sub(1);
            }
            NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(register)) => {
                state.write(
                    self.sp.get(),
                    self.get_16bit_register(register).to_be_bytes()[0],
                );
                *self.sp.get_mut() = self.sp.get().wrapping_sub(1);
            }
            NoRead(WriteLsbPcWhereSpPointsAndLoadCacheToPc) => {
                state.write(self.sp.get(), self.pc.get().to_be_bytes()[1]);
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
                let register_value = self.get_8bit_register(register);
                let decremented = register_value.wrapping_sub(1);
                *self.get_8bit_register_mut(register) = decremented;
                *self.z_flag.get_mut() = decremented == 0;
                *self.n_flag.get_mut() = true;
                *self.h_flag.get_mut() = (register_value & 0x0F) == 0;
            }
            NoRead(Compare) => {
                let (result, carry) = self.a.get().overflowing_sub(self.lsb.get());
                *self.z_flag.get_mut() = result == 0;
                *self.n_flag.get_mut() = true;
                *self.h_flag.get_mut() = (self.a.get() ^ self.lsb.get() ^ result) & 0x10 == 0x10;
                *self.c_flag.get_mut() = carry;
            }
            NoRead(LoadToCachedAddressFromA) => {
                state.write(
                    u16::from_be_bytes([self.msb.get(), self.lsb.get()]),
                    self.a.get(),
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
        let should_load_next_opcode = self.instruction_register.1.is_empty();
        let opcode = state.memory[usize::from(self.pc)];

        let inst = self.instruction_register.0;

        let mut write_once = self.write_once();

        let should_increment_pc = matches!(
            inst,
            Instruction::Read(ReadAddress::Register(Register16Bit::PC), _)
        );

        print!("Executing {inst:?}");

        let inst = match inst {
            Instruction::NoRead(no_read) => AfterReadInstruction::NoRead(no_read),
            Instruction::Read(ReadAddress::Accumulator, read) => AfterReadInstruction::Read(
                state.memory[usize::from(0xff00 | (write_once.lsb.get() as u16))],
                read,
            ),
            Instruction::Read(ReadAddress::Register(register), read) => AfterReadInstruction::Read(
                state.memory[usize::from(write_once.get_16bit_register(register))],
                read,
            ),
        };

        if let AfterReadInstruction::Read(value, _) = inst {
            print!(", read: 0x{value:x}");
        }

        println!();

        move |state| {
            if should_increment_pc {
                *write_once.pc.get_mut() = write_once.pc.get().wrapping_add(1);
            }

            match write_once.execute_instruction(state, inst) {
                PipelineAction::Pop => write_once.pipeline_pop_front(),
                PipelineAction::Replace(instructions) => {
                    *write_once.instruction_register.get_mut() = instructions
                }
            }

            if should_load_next_opcode {
                println!(
                    "Read opcode at ${:04x} (0x{opcode:02x})",
                    write_once.pc.get()
                );
                *write_once.instruction_register.get_mut() =
                    get_instructions(opcode, write_once.is_cb_mode.get());
                *write_once.is_cb_mode.get_mut() = opcode == 0xcb;
                *write_once.pc.get_mut() = write_once.pc.get().wrapping_add(1);
            }
        }
    }
}

impl<T: StateMachine, U: StateMachine> StateMachine for (T, U) {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        let first = self.0.execute(state);
        let second = self.1.execute(state);
        move |mut state| {
            first(state.reborrow());
            second(state);
        }
    }
}
