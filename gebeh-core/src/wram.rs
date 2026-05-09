use core::ops::{Deref, DerefMut};

use crate::Ram;

const WRAM_BANK_SIZE: u16 = 0x1000;

#[derive(Clone)]
pub struct DmgWram([u8; WRAM_BANK_SIZE as usize * 2]);

impl Default for DmgWram {
    fn default() -> Self {
        Self([0; _])
    }
}

impl Deref for DmgWram {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DmgWram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Ram for DmgWram {}

#[derive(Clone)]
pub struct CgbWram {
    bank: u8,
    data: [u8; WRAM_BANK_SIZE as usize * 8],
}

impl Default for CgbWram {
    fn default() -> Self {
        Self {
            bank: 0,
            data: [0; _],
        }
    }
}

impl CgbWram {
    fn get_address(&self, address: u16) -> usize {
        usize::from(self.bank.max(1)) * 0x1000 + usize::from(address)
    }
}

impl Deref for CgbWram {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let base = self.get_address(0);
        &self.data[base..base + usize::from(WRAM_BANK_SIZE)]
    }
}

impl DerefMut for CgbWram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let base = self.get_address(0);
        &mut self.data[base..base + usize::from(WRAM_BANK_SIZE)]
    }
}

impl Ram for CgbWram {}

pub trait Wram: Ram {
    fn write_bank(&mut self, bank: u8);
    fn read_bank(&self) -> u8;
}

impl Wram for CgbWram {
    fn write_bank(&mut self, bank: u8) {
        self.bank = bank & 0x07;
    }
    fn read_bank(&self) -> u8 {
        self.bank | 0b11111000
    }
}

impl Wram for DmgWram {
    fn write_bank(&mut self, _: u8) {}
    fn read_bank(&self) -> u8 {
        0xff
    }
}
