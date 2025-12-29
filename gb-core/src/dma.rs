use core::ops::Range;

use crate::{
    mbc::Mbc,
    state::{MmuExt, State},
};

// about conflicts
// https://github.com/Gekkio/mooneye-gb/issues/39#issuecomment-265953981

#[derive(Clone)]
pub struct Dma(Range<u16>);

impl Default for Dma {
    fn default() -> Self {
        Self(0..0)
    }
}

impl Dma {
    pub fn execute(&mut self, state: &mut State, mbc: &dyn Mbc, _: u64) {
        if let Some(address) = self.0.next() {
            state.is_dma_active = true;
            state.oam[usize::from(address as u8)] = state.read(address, mbc);
        } else {
            state.is_dma_active = false;
        }

        if state.dma_request {
            state.dma_request = false;
            // for next cycle
            *self = Self(
                u16::from_be_bytes([state.dma_register, 0])
                    ..u16::from_be_bytes([state.dma_register, 0xa0]),
            );
        }
    }
}
