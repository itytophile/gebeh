use crate::ppu::Color;

#[derive(Clone, Copy)]
pub struct Scanline([u8; 40]);

impl Scanline {
    pub fn iter_colors(&self) -> impl Iterator<Item = Color> {
        self.0
            .iter()
            .copied()
            .flat_map(|four_pixels| {
                [
                    four_pixels >> 6,
                    four_pixels >> 4,
                    four_pixels >> 2,
                    four_pixels,
                ]
            })
            .map(Color::from)
    }
}

impl Default for Scanline {
    fn default() -> Self {
        Self([0; 40])
    }
}

#[derive(Clone, Default)]
pub struct ScanlineBuilder {
    buffer: Scanline,
    index: u8, // 0 -> 160, if 160 then the scanline is complete
}

impl ScanlineBuilder {
    pub fn push_pixel(&mut self, color: Color) {
        let shift = 6 - (self.index % 4) * 2;
        let pixel = &mut self.buffer.0[usize::from(self.index / 4)];
        *pixel = (u8::from(color) << shift) | (*pixel & !(0b11 << shift));
        self.index += 1;
    }
    pub fn len(&self) -> u8 {
        self.index
    }
    pub fn get_scanline(&self) -> &Scanline {
        &self.buffer
    }
}
