use std::{collections::HashSet, ops::Deref};

use gebeh_core::mbc::{CartridgeType, Mbc, Mbc1, Mbc3, Mbc5, Rtc};

pub trait CloneMbc<'a>: Mbc {
    fn clone_boxed(&self) -> Box<dyn CloneMbc<'a> + 'a>;
}

impl<'a, T: Mbc + Clone + 'a> CloneMbc<'a> for T {
    fn clone_boxed(&self) -> Box<dyn CloneMbc<'a> + 'a> {
        Box::new(self.clone())
    }
}

pub fn get_mbc<
    'a,
    T: Deref<Target = [u8]> + Clone + 'a + Send,
    U: Rtc + Default + Send + Clone + 'a,
>(
    rom: T,
) -> Option<(CartridgeType, Box<dyn CloneMbc<'a> + 'a + Send>)> {
    let cartridge_type = CartridgeType::try_from(rom.get(0x147).copied().unwrap_or(0)).ok()?;
    let mbc: Box<dyn CloneMbc<'a> + Send> = match cartridge_type {
        CartridgeType::RomOnly => Box::new(rom),
        CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => {
            Box::new(Mbc1::new(rom))
        }
        CartridgeType::Mbc3
        | CartridgeType::Mbc3Ram
        | CartridgeType::Mbc3RamBattery
        | CartridgeType::Mbc3TimerBattery
        | CartridgeType::Mbc3TimerRamBattery => Box::new(Mbc3::new(rom, U::default())),
        CartridgeType::Mbc5 | CartridgeType::Mbc5RamBattery => Box::new(Mbc5::new(rom)),
    };
    Some((cartridge_type, mbc))
}

// for sampling reasons we have to generate the noise values at program start
#[derive(Default)]
struct LinearFeedbackShiftRegister(u16);

impl LinearFeedbackShiftRegister {
    fn tick(&mut self, short_mode: bool) -> u8 {
        // https://gbdev.io/pandocs/Audio_details.html#noise-channel-ch4
        let new_value = (self.0 & 1 != 0) == (self.0 & 0b10 != 0);
        self.0 = self.0 & 0x7fff | ((new_value as u16) << 15);
        if short_mode {
            self.0 = self.0 & 0xff7f | ((new_value as u16) << 7)
        }
        let shifted_out = self.0 & 1;
        self.0 >>= 1;
        shifted_out as u8
    }
}

pub fn get_noise(is_short: bool) -> Vec<u8> {
    let mut lfsr = LinearFeedbackShiftRegister::default();
    let mut already_seen = HashSet::new();
    let mut noise = Vec::new();
    while already_seen.insert(lfsr.0) {
        noise.push(lfsr.tick(is_short));
    }
    noise
}

// https://gbdev.io/pandocs/The_Cartridge_Header.html#0134-0143--title
pub fn get_title_from_rom(rom: &[u8]) -> &str {
    let title = &rom[0x134..0x143];
    let end_zero_pos = title
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(title.len());
    str::from_utf8(&title[..end_zero_pos]).unwrap()
}
