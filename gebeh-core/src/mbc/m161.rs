// https://gbdev.io/pandocs/M161.html

use core::ops::Deref;

use crate::{mbc::*, state::*};

#[derive(Clone)]
pub struct M161<T> {
    rom: T,
    rom_bank: u8,
    disable_bank_switch: bool,
}

impl<T: Deref<Target = [u8]>> M161<T> {
    pub fn new(rom: T) -> Self {
        Self {
            rom,
            rom_bank: 0,
            disable_bank_switch: false,
        }
    }
}

impl<T: Deref<Target = [u8]>> Mbc for M161<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..VIDEO_RAM => {
                self.rom[usize::from(self.rom_bank) * usize::from(ROM_BANK_SIZE) * 2
                    + usize::from(index)]
            }
            EXTERNAL_RAM..WORK_RAM => 0xff,
            _ => panic!(),
        }
    }

    fn write(&mut self, index: u16, value: u8) {
        match index {
            ROM_BANK..VIDEO_RAM => {
                if self.disable_bank_switch {
                    return;
                }
                self.rom_bank = value & 0x07;
                self.disable_bank_switch = true;
            }
            EXTERNAL_RAM..WORK_RAM => {}
            _ => panic!(),
        }
    }

    fn load_saved_ram(&mut self, _: &[u8]) {}

    fn load_additional_data(&mut self, _: &[u8]) {}

    fn get_ram_to_save(&self) -> Option<&[u8]> {
        None
    }

    fn get_additional_data_to_save(&self, _: &mut [u8]) -> usize {
        0
    }

    fn get_rom(&self) -> &[u8] {
        &self.rom
    }
}
