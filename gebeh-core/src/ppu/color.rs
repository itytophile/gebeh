#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ColorIndex {
    Zero,
    One,
    Two,
    Three,
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

    pub fn get_color(self, palette: u8) -> Color {
        let shift: u8 = match self {
            ColorIndex::Zero => 0,
            ColorIndex::One => 2,
            ColorIndex::Two => 4,
            ColorIndex::Three => 6,
        };
        match (palette >> shift) & 0b11 {
            0 => Color::White,
            1 => Color::LightGray,
            2 => Color::DarkGray,
            _ => Color::Black,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Color {
    White,
    LightGray,
    DarkGray,
    Black,
}

impl From<Color> for u8 {
    fn from(value: Color) -> Self {
        match value {
            Color::White => 0b11,
            Color::LightGray => 0b10,
            Color::DarkGray => 0b01,
            Color::Black => 0b00,
        }
    }
}

impl From<Color> for u32 {
    fn from(c: Color) -> u32 {
        match c {
            Color::White => 0xffffff,
            Color::LightGray => 0xaaaaaa,
            Color::DarkGray => 0x555555,
            Color::Black => 0,
        }
    }
}

impl From<Color> for [u8; 4] {
    fn from(c: Color) -> Self {
        match c {
            Color::White => [0xff; 4],
            Color::LightGray => [0xaa, 0xaa, 0xaa, 0xff],
            Color::DarkGray => [0x55, 0x55, 0x55, 0xff],
            Color::Black => [0, 0, 0, 0xff],
        }
    }
}

impl From<u8> for Color {
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
