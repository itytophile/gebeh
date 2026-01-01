use crate::{mbc::*, state::*};
use core::ops::Deref;

#[derive(Clone)]
pub struct Mbc5<T> {
    rom: T,
    rom_offset: usize,
    // 128 KiB
    ram: [u8; 0x20000],
    ram_offset: u16,
    ram_enabled: bool,
    ram_bank_count: u8,
}

impl<T: Deref<Target = [u8]>> Mbc5<T> {
    pub fn new(rom: T) -> Self {
        Self {
            ram_bank_count: get_factor_8_kib_ram(rom.deref()),
            rom,
            rom_offset: usize::from(ROM_BANK_SIZE),
            ram_offset: 0,
            ram: [0; 0x20000],
            ram_enabled: false,
        }
    }

    pub fn set_rom_bank(&mut self, rom_bank: u16) {
        self.rom_offset = rom_bank as usize * usize::from(ROM_BANK_SIZE);
    }
    pub fn set_ram_bank(&mut self, ram_bank: u8) {
        self.ram_offset = u16::from(ram_bank) * RAM_BANK_SIZE;
    }
    pub fn rom_bank(&self) -> u16 {
        (self.rom_offset / usize::from(ROM_BANK_SIZE)) as u16
    }
}

impl<T: Deref<Target = [u8]>> Mbc for Mbc5<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            // 0x0000-0x3FFF - ROM bank 00
            0x0000..=0x3fff => self.rom[usize::from(index)],
            // 0x4000-0x7FFF - ROM bank 00-1FF
            0x4000..=0x7fff => self.rom[self.rom_offset + (index - 0x4000) as usize],
            EXTERNAL_RAM..WORK_RAM => {
                if !self.ram_enabled {
                    return 0xff;
                }
                self.ram[usize::from(self.ram_offset) + (index - 0xa000) as usize]
            }
            _ => 0xff,
        }
    }
    fn write(&mut self, index: u16, value: u8) {
        match index {
            // 0x0000-0x1FFF - RAM enabled flag
            0x0000..=0x1fff => {
                self.ram_enabled = (value & 0x0f) == 0x0a;
            }
            // 0x2000-0x2FFF - ROM bank selection 8 lower bits
            0x2000..=0x2fff => {
                // panic!("ROM BANK SELECTION 1 0x{value:02x}");
                let rom_bank = value as u16;
                self.set_rom_bank(rom_bank);
            }
            // 0x3000-0x3FFF - ROM bank selection 9th bit
            0x3000..=0x3fff => {
                // panic!("ROM BANK SELECTION 2 0x{value:02x}");
                let rom_bank = (self.rom_bank() & 0x00ff) + (((value & 0x01) as u16) << 8);
                self.set_rom_bank(rom_bank);
            }
            // 0x4000-0x5FFF - RAM bank selection
            0x4000..=0x5fff => {
                let ram_bank = value & 0x0f;

                if ram_bank >= self.ram_bank_count {
                    return;
                }

                self.set_ram_bank(ram_bank);
            }
            EXTERNAL_RAM..WORK_RAM => {
                if !self.ram_enabled {
                    return;
                }
                self.ram[usize::from(self.ram_offset) + (index - 0xa000) as usize] = value;
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
