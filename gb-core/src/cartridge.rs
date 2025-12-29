// Mbc implementations stolen from https://github.com/joamag/boytacean

use core::ops::Deref;

use crate::{
    get_factor_8_kib_ram, get_factor_32_kib_rom,
    mbc::Mbc,
    state::{EXTERNAL_RAM, ROM_BANK, SWITCHABLE_ROM_BANK, VIDEO_RAM, WORK_RAM},
};

#[derive(Debug, Clone, Copy)]
pub enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
    Mbc3RamBattery,
    Mbc5RamBattery,
}

impl TryFrom<u8> for CartridgeType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::RomOnly),
            1 => Ok(Self::Mbc1),
            2 => Ok(Self::Mbc1Ram),
            0x13 => Ok(Self::Mbc3RamBattery),
            0x1b => Ok(Self::Mbc5RamBattery),
            _ => Err(value),
        }
    }
}

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

#[derive(Clone)]
pub struct Mbc1<T> {
    rom: T,
    rom_offset: usize,
    // 32 KiB
    ram: [u8; 0x8000],
    ram_offset: u16,
    ram_enabled: bool,
    rom_bank_count: u8,
    ram_bank_count: u8,
}

pub const ROM_BANK_SIZE: u16 = 16384;
pub const RAM_BANK_SIZE: u16 = 8192;

impl<T: Deref<Target = [u8]>> Mbc1<T> {
    pub fn new(rom: T) -> Self {
        Self {
            rom_bank_count: u8::try_from(get_factor_32_kib_rom(rom.deref())).unwrap() * 2,
            ram_bank_count: get_factor_8_kib_ram(rom.deref()),
            rom,
            rom_offset: SWITCHABLE_ROM_BANK.into(),
            ram_offset: 0,
            ram: [0; 0x8000],
            ram_enabled: false,
        }
    }

    pub fn set_rom_bank(&mut self, rom_bank: u8) {
        self.rom_offset = usize::from(rom_bank) * usize::from(ROM_BANK_SIZE);
    }
    pub fn set_ram_bank(&mut self, ram_bank: u8) {
        self.ram_offset = u16::from(ram_bank) * RAM_BANK_SIZE;
    }
}

impl<T: Deref<Target = [u8]>> Mbc for Mbc1<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => self.rom[usize::from(index)],
            SWITCHABLE_ROM_BANK..VIDEO_RAM => {
                self.rom[self.rom_offset - usize::from(SWITCHABLE_ROM_BANK) + usize::from(index)]
            }
            EXTERNAL_RAM..WORK_RAM => self.ram[usize::from(self.ram_offset - EXTERNAL_RAM + index)],
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
                let mut rom_bank = value & 0x1f;
                rom_bank &= self.rom_bank_count * 2 - 1;
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
                    panic!("Attempt to write to ERAM while write protect is active");
                }
                self.ram[usize::from(self.ram_offset + index - EXTERNAL_RAM)] = value;
            }
            _ => panic!(),
        }
    }
}

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
}
