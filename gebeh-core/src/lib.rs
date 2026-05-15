#![no_std]
#![forbid(unsafe_code)]

use arrayvec::ArrayVec;

use crate::{
    apu::Apu,
    cpu::{
        BOOTIX_BOOT_ROM, CGB_BOOT_ROM, Cpu,
        speed_switch::{CgbSpeedSwitch, SpeedSwitch},
    },
    interrupts::Interrupts,
    joypad::{Joypad, JoypadInput},
    mbc::Mbc,
    ppu::{
        Ppu, StatInterruptWriteQuirk, StatRegisterHandler,
        color_palettes::{ColorPalettes, ColorPalettesRegs},
        hdma::{Hdma, HdmaRegs},
        object_priority_mode::{ObjectPriorityMode, ObjectPriorityModeRegs},
        renderer::{CgbRenderer, DmgRenderer, Renderer},
        scanline::{DmgScanlineBuilder, ScanlineBuilder},
        vram::{CgbVram, DmgVram, VramRegs},
    },
    serial::{CgbSerial, DmgSerial, Serial},
    timer::Timer,
    wram::{CgbWram, DmgWram, Wram},
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

pub trait Ram: Default + Clone + Send + Sync {
    fn read(&self, index: u16) -> u8;
    fn write(&mut self, index: u16, value: u8);
}

pub struct Peripherals<'a, M: Mbc + ?Sized, Mo: Model> {
    pub mbc: &'a mut M,
    pub timer: &'a mut Timer,
    pub joypad: &'a mut Joypad,
    pub apu: &'a mut Apu,
    pub ppu: &'a mut Ppu<Mo>,
    pub serial: &'a mut Mo::Serial,
    pub wram: &'a mut Mo::Wram,
    pub interrupts: &'a mut Interrupts,
    pub hdma: &'a mut Mo::HdmaRegs,
}

impl<M: Mbc + ?Sized, Mo: Model> Peripherals<'_, M, Mo> {
    pub fn get_ref(&self) -> PeripheralsRef<'_, M, Mo> {
        PeripheralsRef {
            mbc: self.mbc,
            timer: self.timer,
            joypad: self.joypad,
            apu: self.apu,
            ppu: self.ppu,
            serial: self.serial,
            wram: self.wram,
            interrupts: *self.interrupts,
            hdma: self.hdma,
        }
    }
}

pub struct PeripheralsRef<'a, M: Mbc + ?Sized, Mo: Model> {
    pub mbc: &'a M,
    pub timer: &'a Timer,
    pub joypad: &'a Joypad,
    pub apu: &'a Apu,
    pub ppu: &'a Ppu<Mo>,
    pub serial: &'a Mo::Serial,
    pub wram: &'a Mo::Wram,
    pub interrupts: Interrupts,
    pub hdma: &'a Mo::HdmaRegs,
}

pub const WIDTH: u8 = 160;
pub const HEIGHT: u8 = 144;
// https://gbdev.io/pandocs/Specifications.html
pub const SYSTEM_CLOCK_FREQUENCY: u32 = 4194304 / 4;

pub trait Model: Clone + 'static {
    type Renderer: Renderer<Self>;
    type StatRegisterHandler: StatRegisterHandler;
    type Wram: Wram;
    type HdmaRegs: HdmaRegs;
    type SpeedSwitch: SpeedSwitch;
    type Serial: Serial;
    type Vram: VramRegs;
    type ColorPalettes: ColorPalettesRegs;
    type ScanlineBuilder: ScanlineBuilder;
    type ObjectPriorityMode: ObjectPriorityModeRegs;
    fn execute<M: Mbc + ?Sized>(emulator: &mut Emulator<Self>, mbc: &mut M) -> Option<u8>
    where
        Self: Sized;
    fn get_emulator() -> Emulator<Self>;
}

#[derive(Clone)]
pub struct Dmg;
#[derive(Clone)]
pub struct Cgb;

impl Model for Dmg {
    type Renderer = DmgRenderer;
    type StatRegisterHandler = StatInterruptWriteQuirk;
    type Wram = DmgWram;
    type HdmaRegs = ();
    type SpeedSwitch = ();
    type Serial = DmgSerial;
    type Vram = DmgVram;
    type ColorPalettes = ();
    type ScanlineBuilder = DmgScanlineBuilder;
    type ObjectPriorityMode = ();
    fn execute<M: Mbc + ?Sized>(emulator: &mut Emulator<Self>, mbc: &mut M) -> Option<u8> {
        emulator.execute(mbc)
    }
    fn get_emulator() -> Emulator<Self> {
        Emulator {
            ppu: Default::default(),
            cpu: Cpu::new(&BOOTIX_BOOT_ROM),
            interrupts: Interrupts::default(),
            timer: Timer::default(),
            joypad: Joypad::default(),
            apu: Apu::default(),
            serial: DmgSerial::default(),
            wram: DmgWram::default(),
            cycles: 0,
            hdma: (),
        }
    }
}

impl Model for Cgb {
    type Renderer = CgbRenderer;
    type StatRegisterHandler = ();
    type Wram = CgbWram;
    type HdmaRegs = Hdma;
    type SpeedSwitch = CgbSpeedSwitch;
    type Serial = CgbSerial;
    type Vram = CgbVram;
    type ColorPalettes = ColorPalettes;
    type ScanlineBuilder = ArrayVec<u16, 160>;
    type ObjectPriorityMode = ObjectPriorityMode;
    fn execute<M: Mbc + ?Sized>(emulator: &mut Emulator<Self>, mbc: &mut M) -> Option<u8> {
        emulator.execute(mbc)
    }
    fn get_emulator() -> Emulator<Self> {
        Emulator {
            ppu: Default::default(),
            cpu: Cpu::new(CGB_BOOT_ROM),
            interrupts: Interrupts::default(),
            timer: Timer::default(),
            joypad: Joypad::default(),
            apu: Apu::default(),
            serial: Default::default(),
            wram: Default::default(),
            cycles: 0,
            hdma: Hdma::default(),
        }
    }
}

#[derive(Clone)]
pub struct Emulator<M: Model> {
    ppu: Ppu<M>,
    cpu: Cpu<M>,
    pub interrupts: Interrupts,
    timer: Timer,
    joypad: Joypad,
    apu: Apu,
    pub serial: M::Serial,
    wram: M::Wram,
    cycles: u64,
    hdma: M::HdmaRegs,
}

impl<M: Model> Default for Emulator<M> {
    fn default() -> Self {
        M::get_emulator()
    }
}

impl<M: Model> Emulator<M> {
    pub fn will_serial_emit_byte(&self) -> bool {
        self.serial
            .will_emit_byte(self.timer.get_system_counter().wrapping_add(1))
    }
    pub fn get_ppu(&self) -> &Ppu<M> {
        &self.ppu
    }
    pub fn get_cpu(&self) -> &Cpu<M> {
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

impl Emulator<Dmg> {
    fn execute<M: Mbc + ?Sized>(&mut self, mbc: &mut M) -> Option<u8> {
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
                hdma: &mut (),
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

impl Emulator<Cgb> {
    fn execute<M: Mbc + ?Sized>(&mut self, mbc: &mut M) -> Option<u8> {
        if self.cpu.speed_switch.contains(CgbSpeedSwitch::DOUBLE_SPEED) {
            return self.double_speed_execute(mbc);
        }

        let ppu_mode = self.ppu.get_ppu_mode();
        let hdma_has_performed =
            self.hdma
                .execute(self.ppu.get_vram_mut(), mbc, &self.wram, ppu_mode)
                | self
                    .hdma
                    .execute(self.ppu.get_vram_mut(), mbc, &self.wram, ppu_mode);

        self.timer.execute(&mut self.interrupts, self.cycles);
        let master_serial_byte = self.serial.execute(
            self.timer.get_system_counter(),
            &mut self.interrupts,
            self.cycles,
        );

        // the apu is slowed down by dividing the div register by two
        let must_increment_div_apu = self.apu.execute(self.timer.get_div() >> 1);

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
        if hdma_has_performed {
            self.ppu.execute_dma(mbc, &self.wram, self.cycles);
        } else {
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
                    hdma: &mut self.hdma,
                },
                self.cycles,
            );
        }

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

    // yea the emulation doesn't speed up. It's not trivial to adapt the network logic for double speed mode.
    // TODO: what about ppu and cpu synchronization?
    fn double_speed_execute<M: Mbc + ?Sized>(&mut self, mbc: &mut M) -> Option<u8> {
        let ppu_mode = self.ppu.get_ppu_mode();

        // hdma is executed only once in a "fast" m-cycle
        let hdma_has_performed =
            self.hdma
                .execute(self.ppu.get_vram_mut(), mbc, &self.wram, ppu_mode);

        self.timer.execute(&mut self.interrupts, self.cycles);
        let master_serial_byte = self.serial.execute(
            self.timer.get_system_counter(),
            &mut self.interrupts,
            self.cycles,
        );
        let must_increment_div_apu = self.apu.execute(self.timer.get_div());

        let interrupts_from_previous_cycle = self.interrupts;

        // ppu is executed only twice in a "fast" m-cycle
        self.ppu.execute(&mut self.interrupts, self.cycles);
        // I don't understand halt timings https://gekkio.fi/blog/2016/game-boy-research-status
        let mut slowed_interrupts_in_halt_mode = None;
        if self.cpu.is_halted {
            slowed_interrupts_in_halt_mode = Some(self.interrupts);
            self.interrupts = interrupts_from_previous_cycle;
        }
        if hdma_has_performed {
            self.ppu.execute_dma(mbc, &self.wram, self.cycles);
        } else {
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
                    hdma: &mut self.hdma,
                },
                self.cycles,
            );
        }

        if let Some(interrupt_flag) = slowed_interrupts_in_halt_mode {
            self.interrupts = interrupt_flag;
        }

        self.ppu.execute(&mut self.interrupts, self.cycles);

        if must_increment_div_apu {
            self.apu.increment_div_apu();
        }
        self.timer.commit_tima_overflow();
        self.cycles = self.cycles.wrapping_add(1);
        master_serial_byte
    }
}

pub trait EmulatorExt {
    fn execute(&mut self, mbc: &mut (impl Mbc + ?Sized)) -> Option<u8>;
}

impl<M: Model> EmulatorExt for Emulator<M> {
    fn execute(&mut self, mbc: &mut (impl Mbc + ?Sized)) -> Option<u8> {
        M::execute(self, mbc)
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
