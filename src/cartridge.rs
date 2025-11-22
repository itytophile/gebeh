use crate::state::{EXTERNAL_RAM, ROM_BANK, SWITCHABLE_ROM_BANK, VIDEO_RAM, WORK_RAM};

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
pub enum Mbc {
    NoMbc,
    Mbc1(Mbc1),
}

pub struct Mbc1 {
    rom: &'static [u8],
    rom_offset: u16,
    // 32 KiB
    ram: [u8; 0x40000],
    ram_offset: u16,
    ram_enabled: bool,
    rom_bank_count: u8,
    ram_bank_count: u8,
}

pub const ROM_BANK_SIZE: u16 = 16384;
pub const RAM_BANK_SIZE: u16 = 8192;

impl Mbc1 {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => self.rom[usize::from(index)],
            SWITCHABLE_ROM_BANK..VIDEO_RAM => {
                self.rom[usize::from(self.rom_offset - SWITCHABLE_ROM_BANK + index)]
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
                self.ram[usize::from(self.ram_offset - EXTERNAL_RAM + index)] = value;
            }
            _ => panic!(),
        }
    }
    pub fn set_rom_bank(&mut self, rom_bank: u8) {
        self.rom_offset = u16::from(rom_bank) * ROM_BANK_SIZE;
    }
    pub fn set_ram_bank(&mut self, ram_bank: u8) {
        self.ram_offset = u16::from(ram_bank) * RAM_BANK_SIZE;
    }
}
