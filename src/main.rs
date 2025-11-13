use arrayvec::ArrayVec;

use crate::state::{
    AfterReadInstruction, Condition, Flag, Instruction, NoReadInstruction, ReadInstruction,
    Register8Bit, Register16Bit, State, WriteOnlyState, get_instructions,
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
    let mut machine = OpCodeFetcher::default().compose(PipelineExecutor::default());

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

#[derive(Default)]
struct OpCodeFetcher {
    is_cb_mode: bool,
}

#[derive(Default)]
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
}

impl PipelineExecutor {
    fn get_8bit_register(&self, register: Register8Bit) -> u8 {
        match register {
            Register8Bit::A => self.a,
            Register8Bit::B => self.b,
            Register8Bit::C => self.c,
            Register8Bit::D => self.d,
            Register8Bit::E => self.e,
            Register8Bit::H => self.h,
            Register8Bit::L => self.l,
            Register8Bit::F => self.get_flag_bits(),
        }
    }

    fn get_8bit_register_mut(&mut self, register: Register8Bit) -> &mut u8 {
        match register {
            Register8Bit::A => &mut self.a,
            Register8Bit::B => &mut self.b,
            Register8Bit::C => &mut self.c,
            Register8Bit::D => &mut self.d,
            Register8Bit::E => &mut self.e,
            Register8Bit::H => &mut self.h,
            Register8Bit::L => &mut self.l,
            Register8Bit::F => unreachable!(),
        }
    }

    fn get_16bit_register(&self, register: Register16Bit) -> u16 {
        match register {
            Register16Bit::AF => u16::from(self.a) << 8 | u16::from(self.get_flag_bits()),
            Register16Bit::BC => u16::from(self.b) << 8 | u16::from(self.c),
            Register16Bit::DE => u16::from(self.d) << 8 | u16::from(self.e),
            Register16Bit::HL => u16::from(self.h) << 8 | u16::from(self.l),
            Register16Bit::SP => self.sp,
        }
    }

    fn set_16bit_register(&mut self, register: Register16Bit, value: u16) {
        let [msb, lsb] = value.to_be_bytes();
        *self.get_8bit_register_mut(register.get_msb()) = msb;
        *self.get_8bit_register_mut(register.get_lsb()) = lsb;
    }

    fn get_flag_bits(&self) -> u8 {
        (self.z_flag as u8) << 7
            | (self.n_flag as u8) << 6
            | (self.h_flag as u8) << 5
            | (self.c_flag as u8) << 4
    }

    fn get_flag(&self, flag: Flag) -> bool {
        match flag {
            Flag::Z => self.z_flag,
            Flag::N => self.n_flag,
            Flag::H => self.h_flag,
            Flag::C => self.c_flag,
        }
    }

    fn execute_instruction(
        &mut self,
        pc: u16,
        mut state: WriteOnlyState,
        inst: AfterReadInstruction,
    ) {
        println!("Executing {inst:?}");
        use AfterReadInstruction::*;
        use NoReadInstruction::*;
        use ReadInstruction::*;

        match inst {
            Read(value, inst) => {
                match inst {
                    ReadIntoLsb => self.lsb = value,
                    ReadIntoMsb => self.msb = value,
                    RelativeJump(condition) => {
                        self.lsb = value;
                        if let Some(Condition { flag, not }) = condition
                            && self.get_flag(flag) != not
                        {
                            state.set_instruction_register((
                                Instruction::NoRead(OffsetPc),
                                // important to match the cycle and not conflict with the overlapping opcode fetch
                                ArrayVec::from_iter([Instruction::NoRead(Nop)]),
                            ));
                        }
                    }
                    PopStackIntoLsb => {
                        self.lsb = value;
                        self.sp = self.sp.wrapping_add(1);
                    }
                    PopStackIntoMsb => {
                        self.msb = value;
                        self.sp = self.sp.wrapping_add(1);
                    }
                }
            }
            NoRead(Nop) => {}
            NoRead(Store8Bit(register)) => {
                *self.get_8bit_register_mut(register) = self.lsb;
            }
            NoRead(Store16Bit(register)) => match register {
                Register16Bit::SP => {
                    self.sp = u16::from_be_bytes([self.msb, self.lsb]);
                }
                Register16Bit::HL => {
                    self.h = self.msb;
                    self.l = self.lsb;
                }
                Register16Bit::AF => unreachable!(),
                Register16Bit::BC => {
                    self.b = self.msb;
                    self.c = self.lsb;
                }
                Register16Bit::DE => {
                    self.d = self.msb;
                    self.e = self.lsb;
                }
            },
            NoRead(Xor(register)) => {
                self.a ^= self.get_8bit_register(register);
                self.z_flag = self.a == 0;
                self.n_flag = false;
                self.h_flag = false;
                self.c_flag = false;
            }
            NoRead(LoadToAddressHlFromADec) => {
                let hl = self.get_16bit_register(Register16Bit::HL);
                state.write(hl, self.a);
                [self.h, self.l] = hl.wrapping_sub(1).to_be_bytes();
            }
            NoRead(LoadToAddressHlFromAInc) => {
                let hl = self.get_16bit_register(Register16Bit::HL);
                state.write(hl, self.a);
                [self.h, self.l] = hl.wrapping_add(1).to_be_bytes();
            }
            NoRead(Bit(bit, register)) => {
                self.z_flag = (self.get_8bit_register(register) & (1 << bit)) == 0;
                self.n_flag = false;
                self.h_flag = true;
            }
            NoRead(OffsetPc) => {
                state.set_pc((pc as i16).wrapping_add(i16::from(self.lsb as i8)) as u16);
            }
            NoRead(LoadFromAccumulator(register)) => {
                state.write(
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
                let register_value = self.get_8bit_register(register);
                let incremented = register_value.wrapping_add(1);
                *self.get_8bit_register_mut(register) = incremented;
                self.z_flag = incremented == 0;
                self.n_flag = false;
                self.h_flag = (register_value & 0x0F) == 0x0F;
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
                self.sp = self.sp.wrapping_sub(1);
            }
            NoRead(WriteMsbOfRegisterWhereSpPointsAndDecSp(register)) => {
                state.write(
                    self.sp,
                    register
                        .map(|reg| self.get_16bit_register(reg))
                        .unwrap_or(pc)
                        .to_be_bytes()[0],
                );
                self.sp = self.sp.wrapping_sub(1);
            }
            NoRead(WriteLsbPcWhereSpPointsAndLoadCacheToPc) => {
                state.write(self.sp, pc.to_be_bytes()[1]);
                state.set_pc(u16::from_be_bytes([self.msb, self.lsb]));
            }
            NoRead(Load { to, from }) => {
                *self.get_8bit_register_mut(to) = self.get_8bit_register(from);
            }
            NoRead(Rl(register)) => {
                let register_value = self.get_8bit_register(register);
                let new_carry = (register_value & 0x80) != 0;
                let new_value = (register_value << 1) | (self.c_flag as u8);
                *self.get_8bit_register_mut(register) = new_value;
                self.z_flag = new_value == 0;
                self.n_flag = false;
                self.h_flag = false;
                self.c_flag = new_carry;
            }
            NoRead(Rla) => {
                let new_carry = (self.a & 0x80) != 0;
                self.a = (self.a << 1) | (self.c_flag as u8);
                self.z_flag = false; // difference with rl r
                self.n_flag = false;
                self.h_flag = false;
                self.c_flag = new_carry;
            }
            NoRead(Dec(register)) => {
                let register_value = self.get_8bit_register(register);
                let decremented = register_value.wrapping_sub(1);
                *self.get_8bit_register_mut(register) = decremented;
                self.z_flag = decremented == 0;
                self.n_flag = true;
                self.h_flag = (register_value & 0x0F) == 0x0F;
            }
        }
    }
}

impl StateMachine for OpCodeFetcher {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        // we load the next opcode if there is only one instruction left in the pipeline
        let should_load_next_opcode = state.instruction_register.1.is_empty();
        let pc = state.pc;
        let opcode = state.memory[usize::from(pc)];
        // Every write here
        move |mut state| {
            if should_load_next_opcode {
                println!("Read opcode at ${pc:04x} (0x{opcode:02x})");
                state.set_instruction_register(get_instructions(opcode, self.is_cb_mode));
                self.is_cb_mode = opcode == 0xcb;
                state.set_pc(pc + 1);
            }
        }
    }
}

impl StateMachine for PipelineExecutor {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        let inst = state.instruction_register.0;
        // if it is the last instruction then the opcode fetcher will override the instruction
        // register concurrently so the pipeline executor should not pop it.
        let should_pop = !state.instruction_register.1.is_empty();
        let pc = state.pc;

        let should_increment_pc = matches!(inst, Instruction::Read(None, _));

        let inst = match inst {
            Instruction::NoRead(no_read) => AfterReadInstruction::NoRead(no_read),
            Instruction::Read(register, read) => AfterReadInstruction::Read(
                state.memory[usize::from(
                    register
                        .map(|reg| self.get_16bit_register(reg))
                        .unwrap_or(pc),
                )],
                read,
            ),
        };

        move |mut state| {
            if should_pop {
                state.pipeline_pop_front();
            }
            if should_increment_pc {
                state.set_pc(pc + 1);
            }
            self.execute_instruction(pc, state, inst);
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
