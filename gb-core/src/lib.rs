#![no_std]

use crate::state::{State, WriteOnlyState};

pub mod cartridge;
pub mod cpu;
pub mod dma;
pub mod ic;
pub mod instructions;
pub mod ppu;
pub mod state;
pub mod timer;

pub const WIDTH: u8 = 160;
pub const HEIGHT: u8 = 144;

pub fn get_factor_32_kib_rom(rom: &[u8]) -> u16 {
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

pub trait StateMachine: Clone {
    /// must take one M-cycle
    fn execute(&mut self, state: &mut State);
    fn compose<T: StateMachine>(self, other: T) -> (Self, T)
    where
        Self: Sized,
    {
        (self, other)
    }
}

impl<T: StateMachine, U: StateMachine> StateMachine for (T, U) {
    fn execute(&mut self, state: &mut State) {
        self.0.execute(state);
        self.1.execute(state);
    }
}
