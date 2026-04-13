use crate::{mbc::*, state::*};
use core::{num::NonZeroU8, ops::Deref};

#[derive(Clone)]
pub struct Mbc1M<T>(Mbc1<T>);

impl<T: Deref<Target = [u8]>> Mbc1M<T> {
    pub fn new(rom: T) -> Self {
        Self(Mbc1::new(rom))
    }
}

impl<T: Deref<Target = [u8]>> Mbc for Mbc1M<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => match self.0.banking_mode {
                BankingMode::Simple => self.0.rom[usize::from(index)],
                BankingMode::Advanced => {
                    self.0.rom[(usize::from(self.0.advanced_bank) << 18) | usize::from(index)]
                }
            },
            SWITCHABLE_ROM_BANK..VIDEO_RAM => {
                self.0.rom[(usize::from(self.0.advanced_bank) << 18)
                    | (usize::from(self.0.rom_bank.get() & 0x0f) << 14)
                    | (usize::from(index) - usize::from(SWITCHABLE_ROM_BANK))]
            }
            EXTERNAL_RAM..WORK_RAM => self.0.read_external_ram(index),
            _ => panic!(),
        }
    }
    fn write(&mut self, index: u16, value: u8) {
        match index {
            0x0000..0x2000 => self.0.ram_enabled = (value & 0x0f) == 0x0a,
            0x2000..0x4000 => {
                self.0.rom_bank = NonZeroU8::new(value & 0x1f).unwrap_or(NonZeroU8::MIN)
            }
            0x4000..0x6000 => self.0.advanced_bank = value & 0x03,
            0x6000..0x8000 => self.0.write_banking_mode(value),
            EXTERNAL_RAM..WORK_RAM => self.0.write_external_ram(index, value),
            _ => panic!(),
        }
    }

    fn load_saved_ram(&mut self, save: &[u8]) {
        self.0.load_saved_ram(save);
    }

    fn load_additional_data(&mut self, additional_data: &[u8]) {
        self.0.load_additional_data(additional_data);
    }

    fn get_ram_to_save(&self) -> Option<&[u8]> {
        self.0.get_ram_to_save()
    }

    fn get_additional_data_to_save(&self, buffer: &mut [u8]) -> usize {
        self.0.get_additional_data_to_save(buffer)
    }

    fn get_rom(&self) -> &[u8] {
        self.0.get_rom()
    }
}
