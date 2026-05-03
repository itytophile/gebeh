use crate::{
    Wram,
    addresses::*,
    apu::Apu,
    dma::Dma,
    interrupts::Interrupts,
    joypad::Joypad,
    mbc::Mbc,
    ppu::{Ppu, Vram},
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
    pub fn read<M: Mbc + ?Sized>(&mut self, index: u16, mbc: &M, vram: &Vram, wram: &Wram) -> u8 {
        // if self.is_used_by_dma {
        //     return self.last_value_read;
        // }

        mmu_read(index, mbc, vram, wram)
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
