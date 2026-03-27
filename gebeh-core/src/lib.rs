#![no_std]
#![forbid(unsafe_code)]

use crate::{
    apu::Apu,
    cpu::{Cpu, Peripherals},
    dma::Dma,
    joypad::{Joypad, JoypadInput},
    mbc::Mbc,
    ppu::Ppu,
    serial::Serial,
    state::State,
    timer::Timer,
};

pub mod apu;
pub mod cpu;
pub mod dma;
pub mod joypad;
pub mod mbc;
pub mod ppu;
pub mod serial;
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
    pub serial: Serial,
    cycles: u64,
}

impl Emulator {
    pub fn will_serial_emit_byte(&self) -> bool {
        self.serial
            .will_emit_byte(self.timer.get_system_counter().wrapping_add(1))
    }
    pub fn get_ppu(&self) -> &Ppu {
        &self.ppu
    }
    pub fn get_cpu(&self) -> &Cpu {
        &self.cpu
    }
    pub fn get_joypad_mut(&mut self) -> &mut JoypadInput {
        &mut self.joypad.input
    }
    pub fn get_joypad(&self) -> &JoypadInput {
        &self.joypad.input
    }
    pub fn get_apu(&self) -> &Apu {
        &self.apu
    }
    pub fn get_timer(&self) -> &Timer {
        &self.timer
    }
    pub fn get_cycles(&self) -> u64 {
        self.cycles
    }
}

impl Emulator {
    pub fn execute<M: Mbc + ?Sized>(&mut self, mbc: &mut M) -> Option<u8> {
        self.timer.execute(&mut self.state, self.cycles);
        let master_serial_byte = self.serial.execute(
            self.timer.get_system_counter(),
            &mut self.state,
            self.cycles,
        );
        let must_increment_div_apu = self.apu.execute(self.timer.get_div());

        let interrupts_from_previous_cycle = self.state.interrupt_flag;
        for i in 0..2 {
            self.ppu.execute(&mut self.state, self.cycles, i);
        }
        // I don't understand halt timings https://gekkio.fi/blog/2016/game-boy-research-status
        let mut slowed_interrupts_in_halt_mode = None;
        if self.cpu.is_halted {
            slowed_interrupts_in_halt_mode = Some(self.state.interrupt_flag);
            self.state.interrupt_flag = interrupts_from_previous_cycle;
        }
        self.cpu.execute(
            &mut self.state,
            Peripherals {
                mbc,
                timer: &mut self.timer,
                joypad: &mut self.joypad,
                apu: &mut self.apu,
                ppu: &mut self.ppu,
                dma: &mut self.dma,
                serial: &mut self.serial,
            },
            self.cycles,
        );
        if let Some(interrupt_flag) = slowed_interrupts_in_halt_mode {
            self.state.interrupt_flag = interrupt_flag;
        }
        for i in 2..4 {
            self.ppu.execute(&mut self.state, self.cycles, i);
        }

        if must_increment_div_apu {
            self.apu.increment_div_apu();
        }
        self.timer.commit_tima_overflow();
        self.cycles = self.cycles.wrapping_add(1);
        master_serial_byte
    }
}

#[derive(Default, Clone)]
pub struct FallingEdge(bool);

impl FallingEdge {
    pub fn update(&mut self, value: bool) -> bool {
        let previous = self.0;
        self.0 = value;
        previous && !value
    }
}
