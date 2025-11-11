use arrayvec::ArrayVec;

#[derive(Clone, Copy, Debug)]
pub enum Register {
    A
}

#[derive(Clone, Copy, Default, Debug)]
pub enum Instruction {
    #[default]
    Nop,
    ReadLsb,
    ReadMsb,
    StoreInSP,
    Xor(Register)
}

// always start with nop when cpu boots
// https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#fetch-and-stuff
pub type Instructions = (Instruction, ArrayVec<Instruction, 4>);

pub fn get_instructions(opcode: u8) -> Instructions {
    use Instruction::*;
    match opcode {
        0 => Default::default(),
        // instructions in arrayvec is reversed
        0x31 => (ReadLsb, ArrayVec::from_iter([StoreInSP, ReadMsb])),
        0xaf => (Xor(Register::A), Default::default()),
        _ => panic!("Opcode not implemented: 0x{opcode:x}"),
    }
}

// une instruction prend plusieurs m-cycles
// l'opcode détermine quel instruction exécuter
// À l'exécution du dernier M-cycle d'une instruction, le prochain opcode est chargé en parallèle

pub struct State<'a> {
    pub instruction_register: Instructions,
    pub pc: u16,
    pub rom: &'a [u8],
}

impl<'a> State<'a> {
    pub fn new(rom: &'a [u8]) -> Self {
        Self {
            instruction_register: Default::default(),
            pc: 0,
            rom,
        }
    }
}

pub struct WriteOnlyState<'a, 'b>(&'a mut State<'b>);

impl<'a, 'b> WriteOnlyState<'a, 'b> {
    pub fn new(state: &'a mut State<'b>) -> Self {
        Self(state)
    }
    pub fn reborrow<'c>(&'c mut self) -> WriteOnlyState<'c, 'b>
    where
        'a: 'c,
    {
        WriteOnlyState(&mut *self.0)
    }
    pub fn set_instruction_register(&mut self, instructions: Instructions) {
        self.0.instruction_register = instructions;
    }

    pub fn pipeline_pop_front(&mut self) {
        // does nothing if there is only one instruction inside the pipeline
        // if there is only one instruction then the OpcodeFetcher will override the whole pipeline
        if let Some(next_inst) = self.0.instruction_register.1.pop() {
            self.0.instruction_register.0 = next_inst;
        }
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.0.pc = pc;
    }

    // the rom can't be written over so it's safe to access it in write only mode
    pub fn get_rom(&self) -> &'b [u8] {
        self.0.rom
    }
}
