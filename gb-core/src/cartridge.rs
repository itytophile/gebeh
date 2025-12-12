use crate::{
    get_factor_8_kib_ram, get_factor_32_kib_rom,
    state::{EXTERNAL_RAM, ROM_BANK, SWITCHABLE_ROM_BANK, VIDEO_RAM, WORK_RAM},
};

#[derive(Debug, Clone, Copy)]
pub enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
}

impl TryFrom<u8> for CartridgeType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::RomOnly),
            1 => Ok(Self::Mbc1),
            2 => Ok(Self::Mbc1Ram),
            _ => Err(value),
        }
    }
}

// Memory Bank Controller
#[derive(Clone)]
pub enum Mbc {
    NoMbc(&'static [u8]),
    Mbc1(Mbc1),
}

impl Mbc {
    pub fn new(rom: &'static [u8]) -> Self {
        match CartridgeType::try_from(rom[0x147]).unwrap() {
            CartridgeType::RomOnly => Self::NoMbc(rom),
            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram => Self::Mbc1(Mbc1::new(rom)),
        }
    }
    pub fn read(&self, index: u16) -> u8 {
        match self {
            Mbc::NoMbc(rom) => rom[usize::from(index)],
            Mbc::Mbc1(mbc1) => mbc1.read(index),
        }
    }
    pub fn write(&mut self, index: u16, value: u8) {
        match self {
            Mbc::NoMbc(_) => panic!("Trying to write to rom"),
            Mbc::Mbc1(mbc1) => mbc1.write(index, value),
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
            rom_bank_count: get_factor_32_kib_rom(rom) * 2,
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
            // 0x6000-0x7FFF - ROM mode selection
            0x6000..=0x7fff => {
                if value == 0x1 && self.rom_bank_count > 32 {
                    unimplemented!("Advanced ROM banking mode for MBC1 is not implemented");
                }
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
