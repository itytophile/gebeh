#![no_std]

use core::num::NonZeroU8;

use crate::{
    cpu::Cpu,
    dma::Dma,
    mbc::Mbc,
    ppu::{LyHandler, Ppu, Speeder},
    state::State,
    timer::Timer,
};

pub mod cartridge;
pub mod cpu;
pub mod dma;
pub mod ic;
pub mod instructions;
pub mod mbc;
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

#[derive(Clone)]
pub struct Emulator {
    ly_handler: LyHandler,
    ppu: Speeder,
    dma: Dma,
    cpu: Cpu,
    pub state: State,
}

impl Emulator {
    pub fn get_ppu(&self) -> &Ppu {
        &self.ppu.0
    }
    pub fn get_cpu(&self) -> &Cpu {
        &self.cpu
    }
}

impl Default for Emulator {
    fn default() -> Self {
        Self {
            ly_handler: LyHandler::default(),
            ppu: Speeder(Ppu::default(), NonZeroU8::new(4).unwrap()),
            dma: Default::default(),
            cpu: Default::default(),
            state: Default::default(),
        }
    }
}

impl Emulator {
    pub fn execute(&mut self, mbc: &mut dyn Mbc, cycle_count: u64) {
        self.dma.execute(&mut self.state, mbc, cycle_count);
        self.ly_handler.execute(&mut self.state, cycle_count);
        self.ppu.execute(&mut self.state, cycle_count);
        Timer.execute(&mut self.state, cycle_count);
        self.cpu.execute(&mut self.state, mbc, cycle_count);
    }
}
