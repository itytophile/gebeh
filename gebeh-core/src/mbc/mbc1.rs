use crate::{mbc::*, state::*};
use core::ops::Deref;

#[derive(Clone)]
enum BankingMode {
    Simple,
    Advanced,
}

#[derive(Clone)]
pub struct Mbc1<T> {
    rom: T,
    rom_bank: u8,
    advanced_bank: u8,
    // 32 KiB
    ram: [u8; 0x8000],
    ram_enabled: bool,
    banking_mode: BankingMode,
}

impl<T: Deref<Target = [u8]>> Mbc1<T> {
    pub fn new(rom: T) -> Self {
        Self {
            rom,
            rom_bank: 0,
            advanced_bank: 0,
            ram: [0; 0x8000],
            ram_enabled: false,
            banking_mode: BankingMode::Simple,
        }
    }

    fn get_ram_bank_count(&self) -> u8 {
        get_factor_8_kib_ram(self.rom.deref())
    }

    fn get_rom_bank_count(&self) -> u8 {
        u8::try_from(get_factor_32_kib_rom(self.rom.deref())).unwrap() << 1
    }

    fn get_rom_bank_upper_bits(&self) -> u8 {
        match self.get_rom_bank_count() {
            128 => self.advanced_bank,
            64 => self.advanced_bank & 1,
            _ => 0,
        }
    }

    fn get_switchable_rom_offset(&self) -> usize {
        // https://gbdev.io/pandocs/MBC1.html#20003fff--rom-bank-number-write-only
        // Citation: If this register is set to $00, it behaves as if it is set to $01
        // [...] If the ROM Bank Number is set to a higher value than the number of banks in the cart,
        // the bank number is masked to the required number of bits.
        // [...] As a result if the ROM is 256 KiB or smaller, it is possible to map bank 0
        // to the 4000–7FFF region — by setting the 5th bit to 1 it will prevent the 00→01 translation
        let rom_bank_number = (self.get_rom_bank_upper_bits() << 5)
            | (self.rom_bank.max(1) & (self.get_rom_bank_count() - 1));
        usize::from(rom_bank_number) * usize::from(ROM_BANK_SIZE)
    }

    fn get_ram_offset(&self) -> usize {
        match self.banking_mode {
            BankingMode::Advanced if self.get_ram_bank_count() == 4 => {
                usize::from(self.advanced_bank) * usize::from(RAM_BANK_SIZE)
            }
            _ => 0,
        }
    }

    fn get_rom_offset(&self) -> usize {
        match self.banking_mode {
            BankingMode::Advanced => {
                (usize::from(self.get_rom_bank_upper_bits()) << 5) * usize::from(ROM_BANK_SIZE)
            }
            BankingMode::Simple => 0,
        }
    }
}

const LIMIT_ROM_BANK_COUNT_BEFORE_ADVANCED: u8 = 32;

impl<T: Deref<Target = [u8]>> Mbc for Mbc1<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => self.rom[usize::from(index) + self.get_rom_offset()],
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
            0x0000..0x2000 => {
                self.ram_enabled = (value & 0x0f) == 0x0a;
            }
            0x2000..0x4000 => self.rom_bank = value & 0x1f,
            0x4000..0x6000 => self.advanced_bank = value & 0x03,
            0x6000..0x8000 => {
                // https://gbdev.io/pandocs/MBC1.html#60007fff--banking-mode-select-write-only
                // Citation: If the cart is not large enough to use the 2-bit register (≤ 8 KiB RAM and ≤ 512 KiB ROM)
                // this mode select has no observable effect
                if self.get_rom_bank_count() <= LIMIT_ROM_BANK_COUNT_BEFORE_ADVANCED
                    && self.get_ram_bank_count() <= 1
                {
                    return;
                }
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
}
