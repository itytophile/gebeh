use crate::ppu::Color;

pub struct Scanline([u8; 40]);

pub struct ScanlineBuilder {
    buffer: [u8; 40],
    index: u8, // 0 -> 160, if 160 then the scanline is complete
}

impl Default for ScanlineBuilder {
    fn default() -> Self {
        Self {
            buffer: [0; 40],
            index: 0,
        }
    }
}

impl ScanlineBuilder {
    pub fn push_pixel(&mut self, color: Color) {
        let shift = 6 - (self.index % 4) * 2;
        let pixel = &mut self.buffer[usize::from(self.index / 4)];
        *pixel = (color.get_bits() << shift) | (*pixel & !(0b11 << shift));
        self.index += 1;
    }
}
