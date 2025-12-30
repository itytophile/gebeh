use crate::{mbc::*, state::*};
use core::ops::Deref;

#[derive(Clone)]
pub struct Mbc3<T> {
    rom: T,
    rom_offset: usize,
    // 32 KiB
    ram: [u8; 0x8000],
    ram_offset: u16,
    ram_enabled: bool,
    rom_bank_count: u8,
    ram_bank_count: u8,
}

impl<T: Deref<Target = [u8]>> Mbc3<T> {
    pub fn new(rom: T) -> Self {
        Self {
            ram_bank_count: get_factor_8_kib_ram(rom.deref()),
            rom_bank_count: u8::try_from(get_factor_32_kib_rom(rom.deref())).unwrap() * 2,
            rom,
            rom_offset: usize::from(ROM_BANK_SIZE),
            ram_offset: 0,
            ram: [0; 0x8000],
            ram_enabled: false,
        }
    }

    pub fn set_ram_bank(&mut self, ram_bank: u8) {
        self.ram_offset = u16::from(ram_bank) * RAM_BANK_SIZE;
    }
    pub fn set_rom_bank(&mut self, rom_bank: u16) {
        self.rom_offset = usize::from(rom_bank) * usize::from(ROM_BANK_SIZE);
    }
}

impl<T: Deref<Target = [u8]>> Mbc for Mbc3<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => self.rom[usize::from(index)],
            SWITCHABLE_ROM_BANK..VIDEO_RAM => *self
                .rom
                .get(self.rom_offset + (index - 0x4000) as usize)
                .unwrap_or(&0x0),
            EXTERNAL_RAM..WORK_RAM => {
                if !self.ram_enabled {
                    return 0xff;
                }
                self.ram[usize::from(self.ram_offset) + (index - 0xa000) as usize]
            }
            _ => panic!(),
        }
    }
    fn write(&mut self, index: u16, value: u8) {
        match index {
            // 0x0000-0x1FFF - RAM enabled flag
            0x0000..=0x1fff => {
                self.ram_enabled = (value & 0x0f) == 0x0a;
            }
            // 0x2000-0x3FFF - ROM bank selection 5 lower bits
            0x2000..=0x3fff => {
                let mut rom_bank = value as u16 & 0x7f;
                rom_bank &= u16::from(self.rom_bank_count) * 2 - 1;
                if rom_bank == 0 {
                    rom_bank = 1;
                }
                self.set_rom_bank(rom_bank);
            }
            // 0x4000-0x5FFF - RAM bank selection and ROM bank selection upper bits
            0x4000..=0x5fff => {
                let ram_bank = value & 0x03;
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
            _ => panic!(),
        }
    }
}
