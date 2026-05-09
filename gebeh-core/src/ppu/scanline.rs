use arrayvec::ArrayVec;

use crate::ppu::color::DmgColor;

#[derive(Clone, Copy)]
pub struct DmgScanline([u8; 40]);

pub trait ColorIterator {
    type Item: Into<[u8; 4]>;
    fn iter_colors(&self) -> impl Iterator<Item = Self::Item>;
}

impl DmgScanline {
    pub fn raw(&self) -> &[u8; 40] {
        &self.0
    }
}

impl ColorIterator for DmgScanline {
    type Item = DmgColor;
    fn iter_colors(&self) -> impl Iterator<Item = DmgColor> {
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
            .map(DmgColor::from)
    }
}

impl Default for DmgScanline {
    fn default() -> Self {
        Self([0; 40])
    }
}

#[derive(Clone, Default)]
pub struct DmgScanlineBuilder {
    buffer: DmgScanline,
    index: u8, // 0 -> 160, if 160 then the scanline is complete
}

impl DmgScanlineBuilder {
    pub fn push_pixel(&mut self, color: DmgColor) {
        let shift = 6 - (self.index % 4) * 2;
        let pixel = &mut self.buffer.0[usize::from(self.index / 4)];
        *pixel = (u8::from(color) << shift) | (*pixel & !(0b11 << shift));
        self.index += 1;
    }
    fn len(&self) -> u8 {
        self.index
    }
    pub fn get_scanline(&self) -> &DmgScanline {
        &self.buffer
    }
}

pub trait ScanlineBuilder {
    type Scanline: Clone;
    fn len(&self) -> u8;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get_scanline(&self) -> &Self::Scanline;
}

impl ScanlineBuilder for DmgScanlineBuilder {
    type Scanline = DmgScanline;
    fn len(&self) -> u8 {
        self.len()
    }
    fn get_scanline(&self) -> &DmgScanline {
        self.get_scanline()
    }
}

impl ScanlineBuilder for ArrayVec<u16, 160> {
    type Scanline = [u16; 160];
    fn len(&self) -> u8 {
        self.len() as u8
    }
    fn get_scanline(&self) -> &[u16; 160] {
        self.as_slice().try_into().unwrap()
    }
}

pub struct CgbColor(u16);

impl From<CgbColor> for [u8; 4] {
    fn from(value: CgbColor) -> Self {
        let r = value.0 >> 11;
        let g = value.0 >> 6 & 0x1f;
        let b = value.0 >> 1 & 0x1f;
        [
            u8::try_from(r * 0xff / 0x1f).unwrap(),
            u8::try_from(g * 0xff / 0x1f).unwrap(),
            u8::try_from(b * 0xff / 0x1f).unwrap(),
            0xff,
        ]
    }
}

impl ColorIterator for [u16; 160] {
    type Item = CgbColor;
    fn iter_colors(&self) -> impl Iterator<Item = CgbColor> {
        self.iter().copied().map(CgbColor)
    }
}
