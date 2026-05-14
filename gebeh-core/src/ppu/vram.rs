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

impl Ram for DmgVram {
    fn read(&self, index: u16) -> u8 {
        self.0[usize::from(index)]
    }

    fn write(&mut self, index: u16, value: u8) {
        self.0[usize::from(index)] = value;
    }
}

#[derive(Clone)]
pub struct CgbVram {
    bank: u8,
    data: [[u8; VRAM_BANK_SIZE]; 2],
}

impl CgbVram {
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

impl Ram for CgbVram {
    fn read(&self, index: u16) -> u8 {
        self.data[usize::from(self.bank)][usize::from(index)]
    }

    fn write(&mut self, index: u16, value: u8) {
        self.data[usize::from(self.bank)][usize::from(index)] = value;
    }
}

pub trait VramRegs: Ram {
    fn write_bank(&mut self, bank: u8);
    fn read_bank(&self) -> u8;
}

impl VramRegs for CgbVram {
    fn write_bank(&mut self, bank: u8) {
        self.bank = bank & 0x01;
    }

    fn read_bank(&self) -> u8 {
        self.bank | 0xfe
    }
}

impl VramRegs for DmgVram {
    fn write_bank(&mut self, _: u8) {}

    fn read_bank(&self) -> u8 {
        0xff
    }
}
