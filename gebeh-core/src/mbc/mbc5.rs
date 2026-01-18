use crate::{mbc::*, state::*};
use core::ops::Deref;

#[derive(Clone)]
pub struct Mbc5<T> {
    rom: T,
    // 9 bits
    rom_bank: u16,
    // $00-$0F
    ram_bank: u8,
    // 128 KiB
    ram: [u8; 0x20000],
    ram_enabled: bool,
}

impl<T: Deref<Target = [u8]>> Mbc5<T> {
    pub fn new(rom: T) -> Self {
        Self {
            rom,
            rom_bank: 1,
            ram_bank: 0,
            ram: [0; 0x20000],
            ram_enabled: false,
        }
    }

    fn get_switchable_rom_offset(&self) -> usize {
        usize::from(self.rom_bank) * usize::from(ROM_BANK_SIZE)
    }

    fn get_ram_offset(&self) -> usize {
        usize::from(self.ram_bank) * usize::from(RAM_BANK_SIZE)
    }
}

impl<T: Deref<Target = [u8]>> Mbc for Mbc5<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => self.rom[usize::from(index)],
            SWITCHABLE_ROM_BANK..VIDEO_RAM => {
                self.rom[self.get_switchable_rom_offset() + usize::from(index)
                    - usize::from(SWITCHABLE_ROM_BANK)]
            }
            EXTERNAL_RAM..WORK_RAM => {
                if !self.ram_enabled {
                    return 0xff;
                }
                self.ram[self.get_ram_offset() + usize::from(index) - usize::from(EXTERNAL_RAM)]
            }
            _ => panic!(),
        }
    }
    fn write(&mut self, index: u16, value: u8) {
        match index {
            0x0000..0x2000 => self.ram_enabled = (value & 0x0f) == 0x0a,
            0x2000..0x3000 => self.rom_bank = u16::from(value),
            0x3000..0x4000 => self.rom_bank = (u16::from(value & 1) << 8) | self.rom_bank & 0xff,
            0x4000..0x6000 => self.ram_bank = value & 0x0f,
            EXTERNAL_RAM..WORK_RAM => {
                if !self.ram_enabled {
                    return;
                }
                self.ram[self.get_ram_offset() + usize::from(index) - usize::from(EXTERNAL_RAM)] =
                    value;
            }
            _ => {}
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
}
