#![no_std]
#![forbid(unsafe_code)]

use crate::{
    apu::Apu,
    cpu::Cpu,
    dma::Dma,
    external_bus::{ExternalBus, Peripherals},
    interrupts::Interrupts,
    joypad::{Joypad, JoypadInput},
    mbc::Mbc,
    ppu::Ppu,
    serial::Serial,
    timer::Timer,
};

pub mod addresses;
pub mod apu;
pub mod cpu;
pub mod dma;
pub mod external_bus;
pub mod interrupts;
pub mod joypad;
pub mod mbc;
pub mod ppu;
pub mod serial;
pub mod timer;

pub const WIDTH: u8 = 160;
pub const HEIGHT: u8 = 144;
// https://gbdev.io/pandocs/Specifications.html
pub const SYSTEM_CLOCK_FREQUENCY: u32 = 4194304 / 4;

use crate::addresses::{ECHO_RAM, WORK_RAM};

pub type Wram = [u8; (ECHO_RAM - WORK_RAM) as usize];

#[derive(Clone)]
pub struct Emulator {
    ppu: Ppu,
    dma: Dma,
    cpu: Cpu,
    pub interrupts: Interrupts,
    timer: Timer,
    joypad: Joypad,
    apu: Apu,
    pub serial: Serial,
    wram: Wram,
    external_bus: ExternalBus,
    cycles: u64,
}

impl Default for Emulator {
    fn default() -> Self {
        Self {
            ppu: Default::default(),
            dma: Default::default(),
            cpu: Default::default(),
            timer: Default::default(),
            joypad: Default::default(),
            apu: Default::default(),
            serial: Default::default(),
            wram: [0; _],
            cycles: Default::default(),
            interrupts: Default::default(),
            external_bus: Default::default(),
        }
    }
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
    // don't call this function multiple times in a cycle with different inputs
    // or it can fire interrupts when it shouldn't
    pub fn set_joypad(&mut self, joypad: JoypadInput) {
        let previous_joypad = self.joypad;
        self.joypad.input = joypad;
        // if some bits went from 1 to 0
        if previous_joypad.get_register() & !self.joypad.get_register() != 0 {
            self.interrupts.insert(Interrupts::JOYPAD);
        }
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
        self.timer.execute(&mut self.interrupts, self.cycles);
        let master_serial_byte = self.serial.execute(
            self.timer.get_system_counter(),
            &mut self.interrupts,
            self.cycles,
        );
        let must_increment_div_apu = self.apu.execute(self.timer.get_div());

        let interrupts_from_previous_cycle = self.interrupts;
        for _ in 0..2 {
            self.ppu.execute(&mut self.interrupts, self.cycles);
        }
        // I don't understand halt timings https://gekkio.fi/blog/2016/game-boy-research-status
        let mut slowed_interrupts_in_halt_mode = None;
        if self.cpu.is_halted {
            slowed_interrupts_in_halt_mode = Some(self.interrupts);
            self.interrupts = interrupts_from_previous_cycle;
        }
        self.cpu.execute(
            &mut self.external_bus,
            Peripherals {
                mbc,
                timer: &mut self.timer,
                joypad: &mut self.joypad,
                apu: &mut self.apu,
                ppu: &mut self.ppu,
                dma: &mut self.dma,
                serial: &mut self.serial,
                wram: &mut self.wram,
                interrupts: &mut self.interrupts,
            },
            self.cycles,
        );
        if let Some(interrupt_flag) = slowed_interrupts_in_halt_mode {
            self.interrupts = interrupt_flag;
        }
        for _ in 2..4 {
            self.ppu.execute(&mut self.interrupts, self.cycles);
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
