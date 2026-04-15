// https://gbdev.gg8.se/wiki/articles/Memory_Bank_Controllers#Wisdom_Tree
use crate::{mbc::*, state::*};
use core::ops::Deref;

#[derive(Clone)]
pub struct WisdomTree<T> {
    pub(crate) rom: T,
    pub(crate) rom_bank: u8,
}

impl<T: Deref<Target = [u8]>> WisdomTree<T> {
    pub fn new(rom: T) -> Self {
        Self { rom, rom_bank: 0 }
    }
}

impl<T: Deref<Target = [u8]>> Mbc for WisdomTree<T> {
    fn read(&self, index: u16) -> u8 {
        // Citation: unlike a usual MBC, it switches the whole 32 kiB ROM area instead of just the $4000-$7FFF area
        match index {
            ROM_BANK..VIDEO_RAM => {
                self.rom[usize::from(index)
                    + usize::from(self.rom_bank) * usize::from(ROM_BANK_SIZE) * 2]
            }
            EXTERNAL_RAM..WORK_RAM => 0xff,
            _ => panic!(),
        }
    }

    fn write(&mut self, index: u16, _: u8) {
        // Citation: use the A7-A0 address lines for selecting a bank instead of the data lines
        // Thus, the value you write is ignored, and the lower 8 bits of the address is used.
        // For example, to select bank $XX, you would write any value to address $YYXX, where $YY is in the range $00-$7F.
        match index {
            ROM_BANK..VIDEO_RAM => self.rom_bank = index as u8,
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
