use crate::addresses::*;

#[derive(Clone)]
pub struct DmgWram([u8; (ECHO_RAM - WORK_RAM) as usize]);

impl Default for DmgWram {
    fn default() -> Self {
        Self([0; _])
    }
}

pub trait Wram: Default {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

impl Wram for DmgWram {
    fn read(&self, address: u16) -> u8 {
        self.0[usize::from(address)]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.0[usize::from(address)] = value;
    }
}

pub struct CgbWram {
    bank: u8,
    data: [u8; (ECHO_RAM - WORK_RAM) as usize / 2 * 8],
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
    pub fn read_bank(&self) -> u8 {
        self.bank
    }
    pub fn set_bank(&mut self, bank: u8) {
        self.bank = bank & 0x07;
    }
    fn get_address(&self, address: u16) -> usize {
        usize::from(self.bank.max(1)) * 0x1000 + usize::from(address)
    }
}

impl Wram for CgbWram {
    fn read(&self, address: u16) -> u8 {
        self.data[self.get_address(address)]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.data[self.get_address(address)] = value;
    }
}
