use crate::{mbc::*, state::*};
use core::ops::Deref;

#[derive(Clone)]
enum Mode {
    Ram,
    Ir,
}

#[derive(Clone)]
pub struct Huc1<T> {
    rom: T,
    rom_bank: u8,
    ram_bank: u8,
    mode: Mode,
    // 32 KiB
    pub(crate) ram: [u8; 0x8000],
}

impl<T: Deref<Target = [u8]>> Huc1<T> {
    pub fn new(rom: T) -> Self {
        Self {
            rom,
            rom_bank: 0,
            ram_bank: 0,
            mode: Mode::Ram,
            ram: [0; 0x8000],
        }
    }

    fn get_ram_offset(&self) -> usize {
        usize::from(RAM_BANK_SIZE) * usize::from(self.ram_bank)
    }

    fn get_rom_offset(&self) -> usize {
        usize::from(ROM_BANK_SIZE) * usize::from(self.rom_bank)
    }

    fn read_external_ram(&self, index: u16) -> u8 {
        self.ram[self.get_ram_offset() + usize::from(index) - usize::from(EXTERNAL_RAM)]
    }

    fn write_external_ram(&mut self, index: u16, value: u8) {
        self.ram[self.get_ram_offset() + usize::from(index) - usize::from(EXTERNAL_RAM)] = value;
    }
}

impl<T: Deref<Target = [u8]>> Mbc for Huc1<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => self.rom[usize::from(index)],
            SWITCHABLE_ROM_BANK..VIDEO_RAM => {
                self.rom
                    [self.get_rom_offset() + usize::from(index) - usize::from(SWITCHABLE_ROM_BANK)]
            }
            EXTERNAL_RAM..WORK_RAM => match self.mode {
                Mode::Ir => 0xc0,
                Mode::Ram => self.read_external_ram(index),
            },
            _ => panic!(),
        }
    }

    fn write(&mut self, index: u16, value: u8) {
        match index {
            0x0000..0x2000 => self.mode = if value == 0x0e { Mode::Ir } else { Mode::Ram },
            0x2000..0x4000 => self.rom_bank = value,
            0x4000..0x6000 => self.ram_bank = value,
            0x6000..0x8000 => {}
            EXTERNAL_RAM..WORK_RAM => {
                if core::matches!(self.mode, Mode::Ram) {
                    self.write_external_ram(index, value);
                }
            }
            _ => panic!(),
        }
    }

    fn load_saved_ram(&mut self, save: &[u8]) {
        let min = save.len().min(self.ram.len());
        self.ram[..min].copy_from_slice(&save[..min]);
    }

    fn load_additional_data(&mut self, _: &[u8]) {}

    fn get_ram_to_save(&self) -> Option<&[u8]> {
        Some(&self.ram)
    }

    fn get_additional_data_to_save(&self, _: &mut [u8]) -> usize {
        0
    }

    fn get_rom(&self) -> &[u8] {
        &self.rom
    }
}
