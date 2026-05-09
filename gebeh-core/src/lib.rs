#![no_std]
#![forbid(unsafe_code)]

use core::ops::{Deref, DerefMut};

use crate::{
    apu::Apu,
    cpu::Cpu,
    interrupts::Interrupts,
    joypad::{Joypad, JoypadInput},
    mbc::Mbc,
    ppu::{Ppu, renderer::DmgRenderer},
    serial::Serial,
    timer::Timer,
    wram::DmgWram,
};

pub mod addresses;
pub mod apu;
pub mod cpu;
pub mod external_bus;
pub mod interrupts;
pub mod joypad;
pub mod mbc;
pub mod ppu;
pub mod serial;
pub mod timer;
pub mod wram;

pub trait Ram: Default + Clone + Deref<Target = [u8]> + DerefMut<Target = [u8]> {}

pub struct Peripherals<'a, M: Mbc + ?Sized, W: Ram> {
    pub mbc: &'a mut M,
    pub timer: &'a mut Timer,
    pub joypad: &'a mut Joypad,
    pub apu: &'a mut Apu,
    pub ppu: &'a mut Ppu<DmgRenderer>,
    pub serial: &'a mut Serial,
    pub wram: &'a mut W,
    pub interrupts: &'a mut Interrupts,
}

impl<M: Mbc + ?Sized, W: Ram> Peripherals<'_, M, W> {
    pub fn get_ref(&self) -> PeripheralsRef<'_, M, W> {
        PeripheralsRef {
            mbc: self.mbc,
            timer: self.timer,
            joypad: self.joypad,
            apu: self.apu,
            ppu: self.ppu,
            serial: self.serial,
            wram: self.wram,
            interrupts: *self.interrupts,
        }
    }
}

pub struct PeripheralsRef<'a, M: Mbc + ?Sized, W: Ram> {
    pub mbc: &'a M,
    pub timer: &'a Timer,
    pub joypad: &'a Joypad,
    pub apu: &'a Apu,
    pub ppu: &'a Ppu<DmgRenderer>,
    pub serial: &'a Serial,
    pub wram: &'a W,
    pub interrupts: Interrupts,
}

pub const WIDTH: u8 = 160;
pub const HEIGHT: u8 = 144;
// https://gbdev.io/pandocs/Specifications.html
pub const SYSTEM_CLOCK_FREQUENCY: u32 = 4194304 / 4;

#[derive(Clone, Default)]
pub struct Emulator {
    ppu: Ppu<DmgRenderer>,
    cpu: Cpu,
    pub interrupts: Interrupts,
    timer: Timer,
    joypad: Joypad,
    apu: Apu,
    pub serial: Serial,
    wram: DmgWram,
    cycles: u64,
}

impl Emulator {
    pub fn will_serial_emit_byte(&self) -> bool {
        self.serial
            .will_emit_byte(self.timer.get_system_counter().wrapping_add(1))
    }
    pub fn get_ppu(&self) -> &Ppu<DmgRenderer> {
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
            Peripherals {
                mbc,
                timer: &mut self.timer,
                joypad: &mut self.joypad,
                apu: &mut self.apu,
                ppu: &mut self.ppu,
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
