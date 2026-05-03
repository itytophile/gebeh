use core::ops::Range;

use crate::{
    mbc::Mbc,
    ppu::Ppu,
    state::{State, mmu_read},
};

// about conflicts
// https://github.com/Gekkio/mooneye-gb/issues/39#issuecomment-265953981

#[derive(Clone)]
pub struct Dma {
    range: Range<u16>,
    is_active: bool,
}

impl Default for Dma {
    fn default() -> Self {
        Self {
            range: 0..0,
            is_active: false,
        }
    }
}

impl Dma {
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn execute<M: Mbc + ?Sized>(&mut self, state: &mut State, mbc: &M, ppu: &mut Ppu, _: u64) {
        if let Some(address) = self.range.next() {
            ppu.get_oam_mut()[usize::from(address as u8)] =
                mmu_read(state, address, mbc, ppu.get_vram());
            self.is_active = true;
        } else {
            self.is_active = false;
        }

        if state.dma_request {
            state.dma_request = false;
            // for next cycle
            self.range = u16::from_be_bytes([state.dma_register, 0])
                ..u16::from_be_bytes([state.dma_register, 0xa0]);
        }
    }
}
