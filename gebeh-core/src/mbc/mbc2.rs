use crate::{mbc::*, state::*};
use core::{num::NonZeroU8, ops::Deref};

#[derive(Clone)]
pub struct Mbc2<T> {
    rom: T,
    rom_bank: NonZeroU8,
    ram: [u8; 0x200],
    ram_enabled: bool,
}

impl<T: Deref<Target = [u8]>> Mbc2<T> {
    pub fn new(rom: T) -> Self {
        Self {
            rom,
            rom_bank: NonZeroU8::MIN,
            ram: [0; 0x200],
            ram_enabled: false,
        }
    }

    fn get_rom_bank_count(&self) -> u8 {
        (get_factor_32_kib_rom(self.rom.deref()) as u8) << 1
    }

    fn get_switchable_rom_offset(&self) -> usize {
        let bank = self.rom_bank.get() & (self.get_rom_bank_count() - 1);
        usize::from(bank) * usize::from(ROM_BANK_SIZE)
    }
}

impl<T: Deref<Target = [u8]>> Mbc for Mbc2<T> {
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

                // Citation: Only the bottom 9 bits of the address are used to index into the internal RAM, so RAM access repeats
                let addr = (index - EXTERNAL_RAM) & 0x01ff;
                self.ram[usize::from(addr)]
            }
            _ => panic!(),
        }
    }

    fn write(&mut self, index: u16, value: u8) {
        match index {
            0x0000..0x4000 => {
                // https://gbdev.io/pandocs/MBC2.html#00003fff--ram-enable-rom-bank-number-write-only
                if (index & 0x0100) == 0 {
                    self.ram_enabled = (value & 0x0f) == 0x0a;
                } else {
                    self.rom_bank = NonZeroU8::new(value & 0x0f).unwrap_or(NonZeroU8::MIN);
                }
            }
            EXTERNAL_RAM..WORK_RAM => {
                if !self.ram_enabled {
                    return;
                }

                let addr = (index - EXTERNAL_RAM) & 0x01ff;
                self.ram[usize::from(addr)] = value | 0xf0;
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

    fn get_rom(&self) -> &[u8] {
        &self.rom
    }
}
