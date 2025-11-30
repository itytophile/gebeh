use bitflags::Flags;

use crate::{
    StateMachine,
    state::{State, WriteOnlyState},
};

pub struct Ppu2;

// Tile data

const TILE_LENGTH: u8 = 16;

type TileVram = [u8; 0x1800];
type Tile = [u8; 16];
type Line = [u8; 2];

pub enum ColorIndex {
    Zero = 0b00,
    One = 0b01,
    Two = 0b10,
    Three = 0b11,
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
}

// https://gbdev.io/pandocs/Tile_Data.html#vram-tile-data
fn get_object_tile(vram: &TileVram, index: u8) -> &Tile {
    let base = usize::from(index) * usize::from(TILE_LENGTH);
    vram[base..base + usize::from(TILE_LENGTH)]
        .try_into()
        .unwrap()
}

fn get_bg_win_tile(vram: &TileVram, index: u8, is_signed_addressing: bool) -> &Tile {
    let base = if is_signed_addressing {
        0x1000usize.strict_add_signed(isize::from(index.cast_signed()) * isize::from(TILE_LENGTH))
    } else {
        usize::from(index) * usize::from(TILE_LENGTH)
    };
    vram[base..base + usize::from(TILE_LENGTH)]
        .try_into()
        .unwrap()
}

// Tile maps

type TileMap = [u8; 0x400]; // 32 * 32 Tile indexes

// OAM

type OAM = [u8; 0xa0];

bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
    pub struct ObjectFlags: u8 {
        const PRIORITY = 1 << 7;
        const Y_FLIP = 1 << 6;
        const X_FLIP = 1 << 5;
        const DMG_PALETTE = 1 << 4;
    }
}

struct ObjectAttribute {
    y: u8,
    x: u8,
    tile_index: u8,
    flags: ObjectFlags,
}

impl From<[u8; 4]> for ObjectAttribute {
    fn from([y, x, tile_index, flags]: [u8; 4]) -> Self {
        Self {
            y,
            x,
            tile_index,
            flags: ObjectFlags::from_bits_retain(flags),
        }
    }
}

impl StateMachine for Ppu2 {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        Some(|_: WriteOnlyState| {})
    }
}
