use arrayvec::ArrayVec;
use ref_cast::RefCast;

use crate::ppu::color::{CgbColor, DmgColor};

#[derive(Clone, Copy)]
pub struct DmgScanline([u8; 40]);

impl DmgScanline {
    pub fn raw(&self) -> &[u8; 40] {
        &self.0
    }
}

impl Scanline for DmgScanline {
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

pub trait Scanline: Copy + Default + Send + Sync + 'static {
    type Item: Into<[u8; 4]>;
    fn iter_colors(&self) -> impl Iterator<Item = Self::Item>;
}

pub trait ScanlineBuilder: Send + Sync {
    type Scanline: Scanline;
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

#[derive(RefCast, Clone, Copy)]
#[repr(transparent)]
pub struct CgbScanline([u16; 160]);

impl Default for CgbScanline {
    fn default() -> Self {
        Self([0; _])
    }
}

impl ScanlineBuilder for ArrayVec<u16, 160> {
    type Scanline = CgbScanline;
    fn len(&self) -> u8 {
        self.len() as u8
    }
    fn get_scanline(&self) -> &CgbScanline {
        CgbScanline::ref_cast(self.as_slice().try_into().unwrap())
    }
}

impl Scanline for CgbScanline {
    type Item = CgbColor;
    fn iter_colors(&self) -> impl Iterator<Item = CgbColor> {
        self.0.iter().copied().map(CgbColor)
    }
}
