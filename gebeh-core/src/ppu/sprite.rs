use crate::ppu::TileAttributes;

#[derive(Clone, Copy)]
pub struct Sprite {
    pub y: u8,
    pub x: u8,
    pub tile_index: u8,
    pub flags: TileAttributes,
}

impl From<[u8; 4]> for Sprite {
    fn from([y, x, tile_index, flags]: [u8; 4]) -> Self {
        Self {
            y,
            x,
            tile_index,
            flags: TileAttributes::from_bits_retain(flags),
        }
    }
}
