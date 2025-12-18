use core::ops::{RangeFrom, RangeInclusive};

use crate::{
    StateMachine,
    state::{State, WriteOnlyState},
};

// about conflicts
// https://github.com/Gekkio/mooneye-gb/issues/39#issuecomment-265953981

#[derive(Clone)]
pub struct Dma(RangeInclusive<u16>, RangeFrom<u8>);

impl Default for Dma {
    fn default() -> Self {
        Self(0..=0, 0..)
    }
}

impl StateMachine for Dma {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        let is_requesting = state.dma_request;
        let is_active = state.is_dma_active;

        if !is_active && !is_requesting {
            return None;
        }

        // https://gbdev.io/pandocs/OAM_DMA_Transfer.html#ff46--dma-oam-dma-source-address--start
        let register = state.dma_register.min(0xdf);

        let source = self.0.next().map(|source| state.mmu().read(source));
        let destination = self.1.next().unwrap();

        // must check now or the DMA will be active one cycle longer
        let is_empty = self.0.is_empty();

        Some(move |mut state: WriteOnlyState| {
            if is_requesting {
                state.set_dma_active(true);
                state.set_dma_request(false);
                // for next cycle
                *self = Self(
                    u16::from_be_bytes([register, 0])..=u16::from_be_bytes([register, 0x9f]),
                    0..,
                );
            } else if is_empty {
                state.set_dma_active(false);
            }

            if let Some(source) = source {
                state.write_to_oam(destination, source);
            }
        })
    }
}
