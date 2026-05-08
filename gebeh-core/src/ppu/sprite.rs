bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
    pub struct ObjectFlags: u8 {
        const PRIORITY = 1 << 7;
        const Y_FLIP = 1 << 6;
        const X_FLIP = 1 << 5;
        const DMG_PALETTE = 1 << 4;
        const CGB_BANK = 1 << 3;
    }
}

impl ObjectFlags {
    pub fn get_cgb_palette_index(&self) -> u8 {
        self.bits() & 0x07
    }
}

#[derive(Clone, Copy)]
pub struct Sprite {
    pub y: u8,
    pub x: u8,
    pub tile_index: u8,
    pub flags: ObjectFlags,
}

impl From<[u8; 4]> for Sprite {
    fn from([y, x, tile_index, flags]: [u8; 4]) -> Self {
        Self {
            y,
            x,
            tile_index,
            flags: ObjectFlags::from_bits_retain(flags),
        }
    }
}
