use arrayvec::ArrayVec;

#[derive(Clone, Copy, Default)]
pub enum Instruction {
    #[default]
    Nop,
}

// always start with nop when cpu boots
// https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#fetch-and-stuff
pub type Instructions = (Instruction, ArrayVec<Instruction, 4>);

pub fn execute(inst: Instruction) {}
pub fn get_instructions(opcode: u8) -> Instructions {
    Default::default()
}

// une instruction prend plusieurs m-cycles
// l'opcode détermine quel instruction exécuter
// À l'exécution du dernier M-cycle d'une instruction, le prochain opcode est chargé en parallèle

#[derive(Default)]
pub struct State {
    pub instruction_register: Instructions,
}

pub struct WriteOnlyState<'a>(&'a mut State);

impl<'a> WriteOnlyState<'a> {
    pub fn new(state: &'a mut State) -> Self {
        Self(state)
    }
    pub fn reborrow<'b>(&'b mut self) -> WriteOnlyState<'b>
    where
        'a: 'b,
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
}
