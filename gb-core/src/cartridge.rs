// Mbc implementations stolen from https://github.com/joamag/boytacean

use crate::{
    get_factor_8_kib_ram, get_factor_32_kib_rom,
    state::{EXTERNAL_RAM, ROM_BANK, SWITCHABLE_ROM_BANK, VIDEO_RAM, WORK_RAM},
};

#[derive(Debug, Clone, Copy)]
pub enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
    Mbc5RamBattery,
}

impl TryFrom<u8> for CartridgeType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::RomOnly),
            1 => Ok(Self::Mbc1),
            2 => Ok(Self::Mbc1Ram),
            0x1b => Ok(Self::Mbc5RamBattery),
            _ => Err(value),
        }
    }
}

// Memory Bank Controller
#[derive(Clone)]
pub enum Mbc {
    NoMbc(&'static [u8]),
    Mbc1(Mbc1),
    Mbc5(Mbc5),
}

impl Mbc {
    pub fn new(rom: &'static [u8]) -> Self {
        match CartridgeType::try_from(rom[0x147]).unwrap() {
            CartridgeType::RomOnly => Self::NoMbc(rom),
            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram => Self::Mbc1(Mbc1::new(rom)),
            CartridgeType::Mbc5RamBattery => Self::Mbc5(Mbc5::new(rom)),
        }
    }
    pub fn read(&self, index: u16) -> u8 {
        match self {
            Mbc::NoMbc(rom) => rom[usize::from(index)],
            Mbc::Mbc1(mbc1) => mbc1.read(index),
            Mbc::Mbc5(mbc5) => mbc5.read(index),
        }
    }
    pub fn write(&mut self, index: u16, value: u8) {
        match self {
            Mbc::NoMbc(_) => {}
            Mbc::Mbc1(mbc1) => mbc1.write(index, value),
            Mbc::Mbc5(mbc5) => mbc5.write(index, value),
        }
    }
}

#[derive(Clone)]
pub struct Mbc1 {
    rom: &'static [u8],
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

impl Mbc1 {
    fn new(rom: &'static [u8]) -> Self {
        Self {
            rom,
            rom_offset: SWITCHABLE_ROM_BANK.into(),
            ram_offset: 0,
            rom_bank_count: u8::try_from(get_factor_32_kib_rom(rom)).unwrap() * 2,
            ram_bank_count: get_factor_8_kib_ram(rom),
            ram: [0; 0x8000],
            ram_enabled: false,
        }
    }
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
    pub fn set_rom_bank(&mut self, rom_bank: u8) {
        self.rom_offset = usize::from(rom_bank) * usize::from(ROM_BANK_SIZE);
    }
    pub fn set_ram_bank(&mut self, ram_bank: u8) {
        self.ram_offset = u16::from(ram_bank) * RAM_BANK_SIZE;
    }
}

#[derive(Clone)]
pub struct Mbc5 {
    rom: &'static [u8],
    rom_bank_number: u16, // 9 bits
    // 128 KiB
    ram: [u8; 0x20000],
    ram_offset: u16,
    ram_enabled: bool,
    rom_bank_count: u16,
    ram_bank_count: u8,
}

impl Mbc5 {
    fn new(rom: &'static [u8]) -> Self {
        Self {
            rom,
            rom_bank_number: 0,
            ram_offset: 0,
            rom_bank_count: get_factor_32_kib_rom(rom) * 2,
            ram_bank_count: get_factor_8_kib_ram(rom),
            ram: [0; 0x20000],
            ram_enabled: false,
        }
    }
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => self.rom[usize::from(index)],
            SWITCHABLE_ROM_BANK..VIDEO_RAM => {
                self.rom
                    [self.get_rom_offset() + usize::from(index) - usize::from(SWITCHABLE_ROM_BANK)]
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
            // https://gbdev.io/pandocs/MBC5.html#2000-2fff---8-least-significant-bits-of-rom-bank-number-write-only
            0x2000..0x3000 => {
                self.rom_bank_number = (self.rom_bank_number & 0xff00) | u16::from(value);
            }
            // https://gbdev.io/pandocs/MBC5.html#3000-3fff---9th-bit-of-rom-bank-number-write-only
            0x3000..0x4000 => {
                self.rom_bank_number = (self.rom_bank_number & 0x00ff) | (u16::from(value) << 8);
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
                if self.ram_enabled {
                    self.ram[usize::from(self.ram_offset + index - EXTERNAL_RAM)] = value;
                }
            }
            _ => {}
        }
    }
    fn get_rom_bank_number(&self) -> u16 {
        (self.rom_bank_number & (0x1ff)).min(self.rom_bank_count - 1)
    }
    pub fn get_rom_offset(&self) -> usize {
        usize::from(self.get_rom_bank_number()) * usize::from(ROM_BANK_SIZE)
    }
    pub fn set_ram_bank(&mut self, ram_bank: u8) {
        self.ram_offset = u16::from(ram_bank) * RAM_BANK_SIZE;
    }
}
