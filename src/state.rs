const RAM_START: u16 = 0x8000;

const DMG_BOOT: [u8; 256] = [
    49, 254, 255, 33, 255, 159, 175, 50, 203, 124, 32, 250, 14, 17, 33, 38, 255, 62, 128, 50, 226,
    12, 62, 243, 50, 226, 12, 62, 119, 50, 226, 17, 4, 1, 33, 16, 128, 26, 205, 184, 0, 26, 203,
    55, 205, 184, 0, 19, 123, 254, 52, 32, 240, 17, 204, 0, 6, 8, 26, 19, 34, 35, 5, 32, 249, 33,
    4, 153, 1, 12, 1, 205, 177, 0, 62, 25, 119, 33, 36, 153, 14, 12, 205, 177, 0, 62, 145, 224, 64,
    6, 16, 17, 212, 0, 120, 224, 67, 5, 123, 254, 216, 40, 4, 26, 224, 71, 19, 14, 28, 205, 167, 0,
    175, 144, 224, 67, 5, 14, 28, 205, 167, 0, 175, 176, 32, 224, 224, 67, 62, 131, 205, 159, 0,
    14, 39, 205, 167, 0, 62, 193, 205, 159, 0, 17, 138, 1, 240, 68, 254, 144, 32, 250, 27, 122,
    179, 32, 245, 24, 73, 14, 19, 226, 12, 62, 135, 226, 201, 240, 68, 254, 144, 32, 250, 13, 32,
    247, 201, 120, 34, 4, 13, 32, 250, 201, 71, 14, 4, 175, 197, 203, 16, 23, 193, 203, 16, 23, 13,
    32, 245, 34, 35, 34, 35, 201, 60, 66, 185, 165, 185, 165, 66, 60, 0, 84, 168, 252, 66, 79, 79,
    84, 73, 88, 46, 68, 77, 71, 32, 118, 49, 46, 50, 0, 62, 255, 198, 1, 11, 30, 216, 33, 77, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 1, 224, 80,
];

pub struct State {
    boot_rom: &'static [u8],
    ram: [u8; 0x10000 - RAM_START as usize],
}

impl Default for State {
    fn default() -> Self {
        Self {
            boot_rom: &DMG_BOOT,
            ram: [0; 0x10000 - RAM_START as usize],
        }
    }
}

impl State {
    pub fn mmu(&self) -> MmuRead<'_> {
        MmuRead(self)
    }
}

use std::ops::Index;

pub struct WriteOnlyState<'a>(&'a mut State);

impl<'a> WriteOnlyState<'a> {
    pub fn new(state: &'a mut State) -> Self {
        Self(state)
    }
    pub fn reborrow<'c>(&'c mut self) -> WriteOnlyState<'c>
    where
        'a: 'c,
    {
        WriteOnlyState(&mut *self.0)
    }
    pub fn mmu(&mut self) -> MmuWrite<'_> {
        MmuWrite(self.0)
    }
}

pub struct MmuRead<'a>(&'a State);

impl Index<u16> for MmuRead<'_> {
    type Output = u8;

    fn index(&self, index: u16) -> &Self::Output {
        match index {
            0..RAM_START => self.0.boot_rom.get(usize::from(index)).unwrap_or(&0),
            RAM_START.. => &self.0.ram[usize::from(index - RAM_START)],
        }
    }
}

pub struct MmuWrite<'a>(&'a mut State);

impl MmuWrite<'_> {
    pub fn write(&mut self, index: u16, value: u8) {
        match index {
            0..RAM_START => panic!("Trying to write to ROM"),
            RAM_START.. => self.0.ram[usize::from(index - RAM_START)] = value,
        }
    }
}
