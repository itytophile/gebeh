#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ColorIndex {
    Zero,
    One,
    Two,
    Three,
}

impl From<ColorIndex> for u8 {
    fn from(value: ColorIndex) -> Self {
        match value {
            ColorIndex::Zero => 0,
            ColorIndex::One => 1,
            ColorIndex::Two => 2,
            ColorIndex::Three => 3,
        }
    }
}

impl ColorIndex {
    pub fn new(least_significant_bit: bool, most_significant_bit: bool) -> Self {
        match (most_significant_bit, least_significant_bit) {
            (true, true) => Self::Three,
            (true, false) => Self::Two,
            (false, true) => Self::One,
            (false, false) => Self::Zero,
        }
    }

    pub fn get_color(self, palette: u8) -> DmgColor {
        let shift: u8 = match self {
            ColorIndex::Zero => 0,
            ColorIndex::One => 2,
            ColorIndex::Two => 4,
            ColorIndex::Three => 6,
        };
        match (palette >> shift) & 0b11 {
            0 => DmgColor::White,
            1 => DmgColor::LightGray,
            2 => DmgColor::DarkGray,
            _ => DmgColor::Black,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum DmgColor {
    White,
    LightGray,
    DarkGray,
    Black,
}

impl From<DmgColor> for u8 {
    fn from(value: DmgColor) -> Self {
        match value {
            DmgColor::White => 0b11,
            DmgColor::LightGray => 0b10,
            DmgColor::DarkGray => 0b01,
            DmgColor::Black => 0b00,
        }
    }
}

impl From<DmgColor> for u32 {
    fn from(c: DmgColor) -> u32 {
        match c {
            DmgColor::White => 0xffffff,
            DmgColor::LightGray => 0xaaaaaa,
            DmgColor::DarkGray => 0x555555,
            DmgColor::Black => 0,
        }
    }
}

impl From<DmgColor> for [u8; 4] {
    fn from(c: DmgColor) -> Self {
        match c {
            DmgColor::White => [0xff; 4],
            DmgColor::LightGray => [0xaa, 0xaa, 0xaa, 0xff],
            DmgColor::DarkGray => [0x55, 0x55, 0x55, 0xff],
            DmgColor::Black => [0, 0, 0, 0xff],
        }
    }
}

impl From<u8> for DmgColor {
    fn from(value: u8) -> Self {
        match value & 0b11 {
            0 => Self::Black,
            0b01 => Self::DarkGray,
            0b10 => Self::LightGray,
            0b11 => Self::White,
            _ => unreachable!(),
        }
    }
}
