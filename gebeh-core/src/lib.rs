#![no_std]
#![forbid(unsafe_code)]

use crate::{
    apu::Apu,
    cpu::{Cpu, Peripherals},
    dma::Dma,
    joypad::{Joypad, JoypadInput},
    mbc::Mbc,
    ppu::Ppu,
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

#[derive(Clone, Default)]
pub struct Emulator {
    ppu: Ppu,
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
        &self.ppu
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

impl Emulator {
    pub fn execute<M: Mbc + ?Sized>(&mut self, mbc: &mut M) {
        self.dma.execute(&mut self.state, mbc, self.cycles);
        for i in 0..3 {
            self.ppu.execute(&mut self.state, self.cycles, i);
        }
        self.timer.execute(&mut self.state, self.cycles);
        let must_increment_div_apu = self.apu.execute(self.timer.get_div());
        self.cpu.execute(
            &mut self.state,
            Peripherals {
                mbc,
                timer: &mut self.timer,
                joypad: &mut self.joypad,
                apu: &mut self.apu,
                ppu: &mut self.ppu,
            },
            self.cycles,
        );
        self.ppu.execute(&mut self.state, self.cycles, 3);
        if must_increment_div_apu {
            self.apu.increment_div_apu();
        }
        self.timer.commit_tima_overflow();
        self.cycles = self.cycles.wrapping_add(1);
    }
}
