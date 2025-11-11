use crate::state::{Instruction, Register, State, WriteOnlyState, get_instructions};
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

    let mut state = State::new(&DMG_BOOT);
    // the machine should not be affected by the composition order
    let mut machine = OpCodeFetcher.compose(PipelineExecutor::default());

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

struct OpCodeFetcher;

#[derive(Default)]
struct PipelineExecutor {
    sp: u16,
    lsb: u8,
    msb: u8,
    a: u8,
    z: bool,
    n: bool,
    h: bool,
    c: bool
}

impl PipelineExecutor {
    fn execute_instruction(&mut self, pc: u16, mut state: WriteOnlyState, inst: Instruction) {
        println!("Executing {inst:?}");
        use Instruction::*;
        match inst {
            Nop => {}
            ReadLsb => {
                self.lsb = state.get_rom()[usize::from(pc)];
                state.set_pc(pc + 1);
            }
            ReadMsb => {
                self.msb = state.get_rom()[usize::from(pc)];
                state.set_pc(pc + 1);
            }
            StoreInSP => {
                self.sp = u16::from_le_bytes([self.lsb, self.msb]);
            }
            Xor(register) => {
                self.a ^= match register {
                    Register::A => self.a
                };
                self.z = self.a == 0;
                self.n = false;
                self.h = false;
                self.c = false;
            }
        }
    }
}

impl StateMachine for OpCodeFetcher {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        // we load the next opcode if there is only one instruction left in the pipeline
        let should_load_next_opcode = state.instruction_register.1.is_empty();
        let pc = state.pc;
        // Every write here
        move |mut state| {
            if should_load_next_opcode {
                println!("Read opcode at 0x{pc:x}");
                state.set_instruction_register(dbg!(get_instructions(
                    state.get_rom()[usize::from(pc)]
                )));
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

        move |mut state| {
            if should_pop {
                state.pipeline_pop_front();
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
