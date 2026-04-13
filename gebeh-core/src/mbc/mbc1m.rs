use crate::{mbc::*, state::*};
use core::{num::NonZeroU8, ops::Deref};

#[derive(Clone)]
enum BankingMode {
    Simple,
    Advanced,
}

#[derive(Clone)]
pub struct Mbc1M<T> {
    rom: T,
    rom_bank: NonZeroU8,
    advanced_bank: u8,
    // 32 KiB
    ram: [u8; 0x8000],
    ram_enabled: bool,
    banking_mode: BankingMode,
}

impl<T: Deref<Target = [u8]>> Mbc1M<T> {
    pub fn new(rom: T) -> Self {
        Self {
            rom,
            rom_bank: NonZeroU8::MIN,
            advanced_bank: 0,
            ram: [0; 0x8000],
            ram_enabled: false,
            banking_mode: BankingMode::Simple,
        }
    }

    fn get_ram_bank_count(&self) -> u8 {
        get_factor_8_kib_ram(self.rom.deref())
    }

    fn get_ram_offset(&self) -> usize {
        match self.banking_mode {
            BankingMode::Advanced if self.get_ram_bank_count() == 4 => {
                usize::from(self.advanced_bank) * usize::from(RAM_BANK_SIZE)
            }
            _ => 0,
        }
    }
}

impl<T: Deref<Target = [u8]>> Mbc for Mbc1M<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => match self.banking_mode {
                BankingMode::Simple => self.rom[usize::from(index)],
                BankingMode::Advanced => {
                    self.rom[(usize::from(self.advanced_bank) << 18) | usize::from(index)]
                }
            },
            SWITCHABLE_ROM_BANK..VIDEO_RAM => {
                self.rom[(usize::from(self.advanced_bank) << 18)
                    | (usize::from(self.rom_bank.get() & 0x0f) << 14)
                    | (usize::from(index) - usize::from(SWITCHABLE_ROM_BANK))]
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
            0x2000..0x4000 => {
                self.rom_bank = NonZeroU8::new(value & 0x1f).unwrap_or(NonZeroU8::MIN)
            }
            0x4000..0x6000 => self.advanced_bank = value & 0x03,
            0x6000..0x8000 => {
                self.banking_mode = if value & 1 == 0 {
                    BankingMode::Simple
                } else {
                    BankingMode::Advanced
                }
            }
            EXTERNAL_RAM..WORK_RAM => {
                if !self.ram_enabled {
                    return;
                }
                self.ram[self.get_ram_offset() + usize::from(index) - usize::from(EXTERNAL_RAM)] =
                    value;
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
