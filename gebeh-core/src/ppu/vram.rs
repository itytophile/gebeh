use core::ops::{Deref, DerefMut};

use crate::Ram;

pub const VRAM_BANK_SIZE: usize = 0x2000;

#[derive(Clone)]
pub struct DmgVram([u8; VRAM_BANK_SIZE]);

impl DmgVram {
    pub fn get_inner(&self) -> &[u8; VRAM_BANK_SIZE] {
        &self.0
    }
}

impl Default for DmgVram {
    fn default() -> Self {
        Self([0; _])
    }
}

impl Deref for DmgVram {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DmgVram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Ram for DmgVram {}

#[derive(Clone)]
pub struct CgbVram {
    bank: u8,
    data: [[u8; VRAM_BANK_SIZE]; 2],
}

impl CgbVram {
    pub fn set_bank(&mut self, bank: u8) {
        self.bank = bank & 0x01;
    }

    pub fn get_inner(&self) -> &[[u8; VRAM_BANK_SIZE]; 2] {
        &self.data
    }
}

impl Default for CgbVram {
    fn default() -> Self {
        Self {
            bank: 0,
            data: [[0; _]; _],
        }
    }
}

impl Deref for CgbVram {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data[usize::from(self.bank)]
    }
}

impl DerefMut for CgbVram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data[usize::from(self.bank)]
    }
}

impl Ram for CgbVram {}
