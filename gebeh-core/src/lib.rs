#![no_std]
#![forbid(unsafe_code)]

use core::num::NonZeroU8;

use crate::{
    apu::Apu,
    cpu::{Cpu, Peripherals},
    dma::Dma,
    joypad::{Joypad, JoypadInput},
    mbc::Mbc,
    ppu::{LyHandler, Ppu, Speeder},
    state::State,
    timer::Timer,
};

pub mod apu;
pub mod cpu;
pub mod dma;
pub mod joypad;
pub mod mbc;
pub mod ppu;
pub mod state;
pub mod timer;

pub const WIDTH: u8 = 160;
pub const HEIGHT: u8 = 144;
// https://gbdev.io/pandocs/Specifications.html
pub const SYSTEM_CLOCK_FREQUENCY: u32 = 4194304 / 4;

#[derive(Clone)]
pub struct Emulator {
    ly_handler: LyHandler,
    ppu: Speeder,
    dma: Dma,
    cpu: Cpu,
    pub state: State,
    timer: Timer,
    joypad: Joypad,
    apu: Apu,
    cycles: u64, // debug purposes
}

impl Emulator {
    pub fn get_ppu(&self) -> &Ppu {
        &self.ppu.0
    }
    pub fn get_cpu(&self) -> &Cpu {
        &self.cpu
    }
    pub fn get_joypad_mut(&mut self) -> &mut JoypadInput {
        &mut self.joypad.input
    }
    pub fn get_apu(&self) -> &Apu {
        &self.apu
    }
    pub fn get_timer(&self) -> &Timer {
        &self.timer
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
            joypad: Default::default(),
            apu: Default::default(),
            cycles: 0,
        }
    }
}

impl Emulator {
    pub fn execute<M: Mbc + ?Sized>(&mut self, mbc: &mut M) {
        self.dma.execute(&mut self.state, mbc, self.cycles);
        self.ly_handler.execute(&mut self.state, self.cycles);
        self.ppu.execute(&mut self.state, self.cycles);
        self.timer.execute(&mut self.state, self.cycles);
        self.apu.execute(self.timer.get_div());
        self.cpu.execute(
            &mut self.state,
            Peripherals {
                mbc,
                timer: &mut self.timer,
                joypad: &mut self.joypad,
                apu: &mut self.apu,
            },
            self.cycles,
        );
        self.timer.commit_tima_overflow();
        self.cycles = self.cycles.wrapping_add(1);
    }
}
