mod mbc1;
mod mbc3;
mod mbc5;

use core::ops::Deref;

pub use mbc1::*;
pub use mbc3::*;
pub use mbc5::*;

// this trait will make people able to build alien MBCs
// It will be used like (&dyn Mbc) to avoid allocating too much on the stack
// if the program doesn't need MBCs with big ram.
// Don't want to do static dispatch to avoid monomorphization
pub trait Mbc {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

impl<T: Deref<Target = [u8]>> Mbc for T {
    fn read(&self, address: u16) -> u8 {
        self[usize::from(address)]
    }

    fn write(&mut self, _: u16, _: u8) {}
}

#[derive(Debug, Clone, Copy)]
pub enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
    Mbc3RamBattery,
    Mbc5RamBattery,
}

impl TryFrom<u8> for CartridgeType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::RomOnly),
            1 => Ok(Self::Mbc1),
            2 => Ok(Self::Mbc1Ram),
            0x13 => Ok(Self::Mbc3RamBattery),
            0x1b => Ok(Self::Mbc5RamBattery),
            _ => Err(value),
        }
    }
}

pub const ROM_BANK_SIZE: u16 = 16384;
pub const RAM_BANK_SIZE: u16 = 8192;

pub fn get_factor_32_kib_rom(rom: &[u8]) -> u16 {
    1 << rom[0x148]
}

// https://gbdev.io/pandocs/The_Cartridge_Header.html#0149--ram-size
pub fn get_factor_8_kib_ram(rom: &[u8]) -> u8 {
    match rom[0x149] {
        0 => 0,
        2 => 1,
        3 => 4,
        4 => 16,
        5 => 8,
        _ => panic!(),
    }
}
