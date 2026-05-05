use core::ops::Range;

use crate::{Wram, external_bus::external_bus_read, mbc::Mbc, ppu::Ppu};

// about conflicts
// https://github.com/Gekkio/mooneye-gb/issues/39#issuecomment-265953981

#[derive(Clone)]
pub struct Dma {
    range: Range<u16>,
    is_active: bool,
    pub dma_register: u8,
    pub dma_request: bool,
}

impl Default for Dma {
    fn default() -> Self {
        Self {
            range: 0..0,
            is_active: false,
            dma_register: 0,
            dma_request: false,
        }
    }
}

impl Dma {
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn execute<M: Mbc + ?Sized>(&mut self, mbc: &M, ppu: &mut Ppu, wram: &Wram, _: u64) {
        if let Some(address) = self.range.next() {
            self.is_active = true;
            ppu.get_oam_mut()[usize::from(address as u8)] =
                external_bus_read(address, mbc, ppu.get_vram(), wram);
        } else {
            self.is_active = false;
        }

        if self.dma_request {
            self.dma_request = false;
            // for next cycle
            self.range = u16::from_be_bytes([self.dma_register, 0])
                ..u16::from_be_bytes([self.dma_register, 0xa0]);
        }
    }
}
