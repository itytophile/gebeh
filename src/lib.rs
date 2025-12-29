use std::ops::Deref;

use gb_core::mbc::*;

pub trait CloneMbc<'a>: Mbc {
    fn clone_boxed(&self) -> Box<dyn CloneMbc<'a> + 'a>;
}

impl<'a, T: Mbc + Clone + 'a> CloneMbc<'a> for T {
    fn clone_boxed(&self) -> Box<dyn CloneMbc<'a> + 'a> {
        Box::new(self.clone())
    }
}

pub fn get_mbc<'a, T: Deref<Target = [u8]> + Clone + 'a>(
    rom: T,
) -> Option<Box<dyn CloneMbc<'a> + 'a>> {
    let mbc: Box<dyn CloneMbc<'a>> =
        match CartridgeType::try_from(rom.get(0x147).copied().unwrap_or(0)).ok()? {
            CartridgeType::RomOnly => Box::new(rom),
            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram => Box::new(Mbc1::new(rom)),
            CartridgeType::Mbc3RamBattery => Box::new(Mbc3::new(rom)),
            CartridgeType::Mbc5RamBattery => Box::new(Mbc5::new(rom)),
        };
    Some(mbc)
}
