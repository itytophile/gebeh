#![no_std]

use core::num::NonZeroU8;

use crate::{
    cpu::{Cpu, Peripherals},
    dma::Dma,
    mbc::Mbc,
    ppu::{LyHandler, Ppu, Speeder},
    state::State,
    timer::Timer,
};

pub mod cpu;
pub mod dma;
pub mod mbc;
pub mod ppu;
pub mod state;
pub mod timer;

pub const WIDTH: u8 = 160;
pub const HEIGHT: u8 = 144;

#[derive(Clone)]
pub struct Emulator {
    ly_handler: LyHandler,
    ppu: Speeder,
    dma: Dma,
    cpu: Cpu,
    pub state: State,
    timer: Timer,
    cycles: u64, // debug purposes
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
            timer: Default::default(),
            cycles: 0,
        }
    }
}

impl Emulator {
    pub fn execute(&mut self, mbc: &mut dyn Mbc) {
        self.dma.execute(&mut self.state, mbc, self.cycles);
        self.ly_handler.execute(&mut self.state, self.cycles);
        self.ppu.execute(&mut self.state, self.cycles);
        self.timer.execute(&mut self.state, self.cycles);
        self.cpu.execute(
            &mut self.state,
            Peripherals {
                mbc,
                timer: &mut self.timer,
            },
            self.cycles,
        );
        self.timer.commit_tima_overflow();
        self.cycles = self.cycles.wrapping_add(1);
    }
}
