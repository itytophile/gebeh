use crate::state::{State, WriteOnlyState};

pub mod cartridge;
pub mod cpu;
pub mod gpu;
pub mod hardware;
pub mod ic;
pub mod instructions;
pub mod ppu;
pub mod state;
pub mod timer;

pub fn get_factor_32_kib_rom(rom: &[u8]) -> u8 {
    1 << rom[0x148]
}

// https://gbdev.io/pandocs/The_Cartridge_Header.html#0149--ram-size
pub fn get_factor_8_kib_ram(rom: &[u8]) -> u8 {
    match rom[0x149] {
        0 => 0,
        2 => 1,
        3 => 4,
        4 => 16,
        5 => 8,
        _ => panic!(),
    }
}

pub trait StateMachine {
    /// must take one M-cycle
    #[must_use]
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a>;
    fn compose<T: StateMachine>(self, other: T) -> (Self, T)
    where
        Self: Sized,
    {
        (self, other)
    }
}

impl<T: StateMachine, U: StateMachine> StateMachine for (T, U) {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        let first = self.0.execute(state);
        let second = self.1.execute(state);
        Some(move |mut state: WriteOnlyState<'_>| {
            if let Some(first) = first {
                first(state.reborrow());
            }
            if let Some(second) = second {
                second(state);
            }
        })
    }
}
