use core::num::NonZeroU8;

use crate::Ram;

const WRAM_BANK_SIZE: u16 = 0x1000;

#[derive(Clone)]
pub struct DmgWram([u8; WRAM_BANK_SIZE as usize * 2]);

impl Default for DmgWram {
    fn default() -> Self {
        Self([0; _])
    }
}

impl Ram for DmgWram {
    fn read(&self, index: u16) -> u8 {
        self.0[usize::from(index)]
    }

    fn write(&mut self, index: u16, value: u8) {
        self.0[usize::from(index)] = value;
    }
}

#[derive(Clone)]
pub struct CgbWram {
    bank: NonZeroU8,
    data: [[u8; WRAM_BANK_SIZE as usize]; 8],
}

impl Default for CgbWram {
    fn default() -> Self {
        Self {
            bank: NonZeroU8::MIN,
            data: [[0; _]; _],
        }
    }
}

impl Ram for CgbWram {
    fn read(&self, index: u16) -> u8 {
        if index < WRAM_BANK_SIZE {
            self.data[0][usize::from(index)]
        } else {
            self.data[usize::from(self.bank.get())][usize::from(index - WRAM_BANK_SIZE)]
        }
    }

    fn write(&mut self, index: u16, value: u8) {
        if index < WRAM_BANK_SIZE {
            self.data[0][usize::from(index)] = value;
        } else {
            self.data[usize::from(self.bank.get())][usize::from(index - WRAM_BANK_SIZE)] = value;
        }
    }
}

pub trait Wram: Ram {
    fn write_bank(&mut self, bank: u8);
    fn read_bank(&self) -> u8;
}

impl Wram for CgbWram {
    fn write_bank(&mut self, bank: u8) {
        self.bank = NonZeroU8::new(bank & 0x07).unwrap_or(NonZeroU8::MIN);
    }
    fn read_bank(&self) -> u8 {
        self.bank.get() | 0b11111000
    }
}

impl Wram for DmgWram {
    fn write_bank(&mut self, _: u8) {}
    fn read_bank(&self) -> u8 {
        0xff
    }
}
