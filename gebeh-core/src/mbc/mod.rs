mod mbc1;
mod mbc3;
mod mbc5;

use core::ops::Deref;

pub use mbc1::*;
pub use mbc3::*;
pub use mbc5::*;

// this trait will make people able to build alien MBCs.
// Be careful about monomorphization
pub trait Mbc {
    // maybe too much responsibility?
    fn load_saved_ram(&mut self, save: &[u8]);
    // useful for RTC at the moment
    fn load_additional_data(&mut self, additional_data: &[u8]);
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
    fn get_ram_to_save(&self) -> Option<&[u8]>;
    /// Returns how many bytes were written into the buffer. Panics if the buffer is not big enough.
    fn get_additional_data_to_save(&self, buffer: &mut [u8]) -> usize;
}

impl<T: Deref<Target = [u8]>> Mbc for T {
    fn read(&self, address: u16) -> u8 {
        self[usize::from(address)]
    }

    fn write(&mut self, _: u16, _: u8) {}
    fn get_ram_to_save(&self) -> Option<&[u8]> {
        None
    }
    fn get_additional_data_to_save(&self, _: &mut [u8]) -> usize {
        0
    }
    fn load_saved_ram(&mut self, _: &[u8]) {}
    fn load_additional_data(&mut self, _: &[u8]) {}
}

#[derive(Debug, Clone, Copy)]
pub enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
    Mbc1RamBattery,
    Mbc3TimerBattery,
    Mbc3TimerRamBattery,
    Mbc3,
    Mbc3Ram,
    Mbc3RamBattery,
    Mbc5,
    Mbc5RamBattery,
}

impl CartridgeType {
    pub fn has_battery(&self) -> bool {
        use CartridgeType::*;
        matches!(
            self,
            Mbc1RamBattery
                | Mbc3TimerBattery
                | Mbc3TimerRamBattery
                | Mbc3RamBattery
                | Mbc5RamBattery
        )
    }
}

impl TryFrom<u8> for CartridgeType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // https://gbdev.io/pandocs/The_Cartridge_Header.html#0147--cartridge-type
        match value {
            0 => Ok(Self::RomOnly),
            1 => Ok(Self::Mbc1),
            2 => Ok(Self::Mbc1Ram),
            3 => Ok(Self::Mbc1RamBattery),
            0x0f => Ok(Self::Mbc3TimerBattery),
            0x10 => Ok(Self::Mbc3TimerRamBattery),
            0x11 => Ok(Self::Mbc3),
            0x12 => Ok(Self::Mbc3Ram),
            0x13 => Ok(Self::Mbc3RamBattery),
            0x19 => Ok(Self::Mbc5),
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
