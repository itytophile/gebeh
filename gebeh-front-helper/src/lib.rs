use std::{collections::HashSet, ops::Deref};

use gebeh_core::mbc::{
    CartridgeType, Huc1, Mbc, Mbc1, Mbc1M, Mbc3, Mbc5, ROM_SIZE_HEADER, Rtc, Tama5, WisdomTree,
};

pub type EasyMbc = Box<dyn CloneMbc<'static>>;

pub trait CloneMbc<'a>: Mbc {
    fn clone_boxed(&self) -> Box<dyn CloneMbc<'a> + 'a>;
}

impl<'a, T: Mbc + Clone + 'a> CloneMbc<'a> for T {
    fn clone_boxed(&self) -> Box<dyn CloneMbc<'a> + 'a> {
        Box::new(self.clone())
    }
}

const LOGO: [u8; 0x30] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

fn get_logo(rom: &[u8]) -> Option<&[u8; 0x30]> {
    rom[0x0104..0x0134].try_into().ok()
}

const MINIMUM_GAMES_COUNT_IN_MULTICART: usize = 3;

fn is_multicart(rom: &[u8]) -> bool {
    // copied from gameroy and adapted
    rom[ROM_SIZE_HEADER] == 5
        && (0..4)
            .filter_map(|i| get_logo(&rom[i * 0x40000..]))
            .filter(|logo| *logo == &LOGO)
            .take(MINIMUM_GAMES_COUNT_IN_MULTICART)
            .count()
            >= MINIMUM_GAMES_COUNT_IN_MULTICART
}

pub fn is_wisdom_tree(rom: &[u8]) -> bool {
    // https://gbdev.gg8.se/wiki/articles/Memory_Bank_Controllers#Wisdom_Tree
    const NEEDLES: [&[u8]; 2] = [b"WISDOM TREE", b"WISDOM\0TREE"];
    NEEDLES
        .iter()
        .any(|needle| rom.windows(needle.len()).any(|w| w == *needle))
}

pub fn get_mbc<'a, T: Deref<Target = [u8]> + Clone + 'a, U: Rtc + Default + Clone + 'a>(
    rom: T,
) -> Option<(CartridgeType, Box<dyn CloneMbc<'a> + 'a>)> {
    let cartridge_type = CartridgeType::try_from(rom.get(0x147).copied().unwrap_or(0)).ok()?;

    if is_wisdom_tree(rom.deref()) {
        return Some((cartridge_type, Box::new(WisdomTree::new(rom))));
    }

    let mbc: Box<dyn CloneMbc<'a>> = match cartridge_type {
        CartridgeType::RomOnly => Box::new(rom),
        CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => {
            if is_multicart(rom.deref()) {
                Box::new(Mbc1M::new(rom))
            } else {
                Box::new(Mbc1::new(rom))
            }
        }
        CartridgeType::Mbc3
        | CartridgeType::Mbc3Ram
        | CartridgeType::Mbc3RamBattery
        | CartridgeType::Mbc3TimerBattery
        | CartridgeType::Mbc3TimerRamBattery => Box::new(Mbc3::new(rom, U::default())),
        CartridgeType::Mbc5 | CartridgeType::Mbc5RamBattery => Box::new(Mbc5::new(rom)),
        CartridgeType::Tama5 => Box::new(Tama5::new(rom)),
        CartridgeType::Huc1 => Box::new(Huc1::new(rom)),
    };
    Some((cartridge_type, mbc))
}

pub fn get_mbc_send<
    'a,
    T: Deref<Target = [u8]> + Clone + Send + 'a,
    U: Rtc + Default + Send + Clone + 'a,
>(
    rom: T,
) -> Option<(CartridgeType, Box<dyn CloneMbc<'a> + Send + 'a>)> {
    let cartridge_type = CartridgeType::try_from(rom.get(0x147).copied().unwrap_or(0)).ok()?;

    if is_wisdom_tree(rom.deref()) {
        return Some((cartridge_type, Box::new(WisdomTree::new(rom))));
    }

    let mbc: Box<dyn CloneMbc<'a> + Send> = match cartridge_type {
        CartridgeType::RomOnly => Box::new(rom),
        CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => {
            if is_multicart(rom.deref()) {
                Box::new(Mbc1M::new(rom))
            } else {
                Box::new(Mbc1::new(rom))
            }
        }
        CartridgeType::Mbc3
        | CartridgeType::Mbc3Ram
        | CartridgeType::Mbc3RamBattery
        | CartridgeType::Mbc3TimerBattery
        | CartridgeType::Mbc3TimerRamBattery => Box::new(Mbc3::new(rom, U::default())),
        CartridgeType::Mbc5 | CartridgeType::Mbc5RamBattery => Box::new(Mbc5::new(rom)),
        CartridgeType::Tama5 => Box::new(Tama5::new(rom)),
        CartridgeType::Huc1 => Box::new(Huc1::new(rom)),
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
