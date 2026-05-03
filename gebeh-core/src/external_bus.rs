use crate::{
    Wram,
    addresses::*,
    apu::Apu,
    cpu::Cpu,
    dma::Dma,
    interrupts::Interrupts,
    joypad::Joypad,
    mbc::Mbc,
    ppu::{LcdStatus, Ppu, Vram},
    serial::Serial,
    timer::Timer,
};

pub struct Peripherals<'a, M: Mbc + ?Sized> {
    pub mbc: &'a mut M,
    pub timer: &'a mut Timer,
    pub joypad: &'a mut Joypad,
    pub apu: &'a mut Apu,
    pub ppu: &'a mut Ppu,
    pub dma: &'a mut Dma,
    pub serial: &'a mut Serial,
    pub wram: &'a mut Wram,
    pub interrupts: &'a mut Interrupts,
}

impl<M: Mbc + ?Sized> Peripherals<'_, M> {
    pub fn get_ref(&self) -> PeripheralsRef<'_, M> {
        PeripheralsRef {
            mbc: self.mbc,
            timer: self.timer,
            joypad: self.joypad,
            apu: self.apu,
            ppu: self.ppu,
            dma: self.dma,
            serial: self.serial,
            wram: self.wram,
            interrupts: *self.interrupts,
        }
    }
}

pub struct PeripheralsRef<'a, M: Mbc + ?Sized> {
    pub mbc: &'a M,
    pub timer: &'a Timer,
    pub joypad: &'a Joypad,
    pub apu: &'a Apu,
    pub ppu: &'a Ppu,
    pub dma: &'a Dma,
    pub serial: &'a Serial,
    pub wram: &'a Wram,
    pub interrupts: Interrupts,
}

#[derive(Clone, Default)]
pub struct ExternalBus {
    last_value_read: u8,
    is_used_by_dma: bool,
}

impl ExternalBus {
    pub fn read<M: Mbc + ?Sized>(
        &mut self,
        index: u16,
        cpu: &Cpu,
        peripherals: PeripheralsRef<M>,
        cycles: u64,
    ) -> u8 {
        if (VIDEO_RAM..EXTERNAL_RAM).contains(&index)
            && peripherals.ppu.get_ppu_mode() == LcdStatus::DRAWING
        {
            return 0xff;
        }

        match index {
            // https://gbdev.io/pandocs/Power_Up_Sequence.html#power-up-sequence
            ..0x100 if !cpu.boot_rom_mapping_control => cpu.boot_rom[usize::from(index)],
            ..OAM => mmu_read(
                index,
                peripherals.mbc,
                peripherals.ppu.get_vram(),
                peripherals.wram,
            ),
            OAM..NOT_USABLE => {
                let ppu = peripherals.ppu.get_ppu_mode();
                if ppu == LcdStatus::DRAWING || ppu == LcdStatus::OAM_SCAN || self.is_used_by_dma {
                    0xff
                } else {
                    peripherals.ppu.get_oam()[usize::from(index - OAM)]
                }
            }
            JOYPAD => peripherals.joypad.get_register(),
            SB => peripherals.serial.sb,
            SC => peripherals.serial.get_control().bits() | 0b01111110,
            0xff03 => 0xff,
            DIV => peripherals.timer.get_div(),
            TIMER_COUNTER => peripherals.timer.get_tima(),
            TIMER_MODULO => peripherals.timer.get_tma(),
            TIMER_CONTROL => peripherals.timer.get_tac(),
            0xff08..INTERRUPT_FLAG => 0xff,
            INTERRUPT_FLAG => peripherals.interrupts.bits() | 0b11100000,
            CH1_SWEEP..LCD_CONTROL => peripherals.apu.read(index, cycles),
            LCD_CONTROL => peripherals.ppu.get_lcd_control().bits(),
            LCD_STATUS => peripherals.ppu.get_lcd_status().bits() | 0b10000000,
            SCY => peripherals.ppu.get_scy(),
            SCX => peripherals.ppu.get_scx(),
            LY => peripherals.ppu.get_ly(),
            LYC => peripherals.ppu.lyc,
            DMA => peripherals.dma.dma_register,
            BGP => peripherals.ppu.get_bgp(),
            OBP0 => peripherals.ppu.get_obp0(),
            OBP1 => peripherals.ppu.get_obp1(),
            WY => peripherals.ppu.get_wy(),
            WX => peripherals.ppu.get_wx(),
            0xff4c => 0xff,
            0xff4d => 0xff,
            0xff4e => 0xff,
            0xff4f => 0xff,
            BOOT_ROM_MAPPING_CONTROL => 0xff,
            0xff51..HRAM => 0xff,
            HRAM..INTERRUPT_ENABLE => cpu.hram[usize::from(index - HRAM)],
            INTERRUPT_ENABLE => cpu.interrupt_enable.bits(),
            _ => todo!("Reading ${index:04x}"),
        }
    }
}

#[derive(Clone)]
pub struct DmaPov;

impl DmaPov {
    pub fn new(bus: &mut ExternalBus) -> Self {
        if bus.is_used_by_dma {
            panic!()
        }
        bus.is_used_by_dma = true;
        DmaPov
    }
    pub fn close(self, bus: &mut ExternalBus) {
        bus.is_used_by_dma = false;
    }
    pub fn read<M: Mbc + ?Sized>(
        &self,
        bus: &mut ExternalBus,
        index: u16,
        mbc: &M,
        vram: &Vram,
        wram: &Wram,
    ) -> u8 {
        let value = mmu_read(index, mbc, vram, wram);
        bus.last_value_read = value;
        value
    }
}

pub fn mmu_read<M: Mbc + ?Sized>(index: u16, mbc: &M, vram: &Vram, wram: &Wram) -> u8 {
    match index {
        0..VIDEO_RAM => mbc.read(index),
        VIDEO_RAM..EXTERNAL_RAM => vram[usize::from(index - VIDEO_RAM)],
        EXTERNAL_RAM..WORK_RAM => mbc.read(index),
        WORK_RAM..ECHO_RAM => wram[usize::from(index - WORK_RAM)],
        // if greater than 0xdfff then the dma has access to a bigger echo ram than the cpu
        // from https://github.com/Gekkio/mooneye-gb/blob/3856dcbca82a7d32bd438cc92fd9693f868e2e23/core/src/hardware.rs#L215
        ECHO_RAM.. => wram[usize::from(index - ECHO_RAM)],
    }
}
