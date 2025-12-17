use core::ops::{RangeFrom, RangeInclusive};

use crate::{
    StateMachine,
    state::{DmaState, State, WriteOnlyState},
};

// about conflicts
// https://github.com/Gekkio/mooneye-gb/issues/39#issuecomment-265953981

#[derive(Default, Clone)]
pub struct Dma(Option<(RangeInclusive<u16>, RangeFrom<u8>)>);

impl StateMachine for Dma {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        // https://gbdev.io/pandocs/OAM_DMA_Transfer.html#ff46--dma-oam-dma-source-address--start
        if self.0.is_none() && state.dma_state != DmaState::Starting {
            return None;
        }

        let mmu = state.mmu();
        let register = state.dma_register.min(0xdf);

        let (source, destination) = if state.dma_state == DmaState::Starting {
            self.0.insert((
                u16::from_be_bytes([register, 0])..=u16::from_be_bytes([register, 0x9f]),
                0..,
            ))
        } else {
            self.0.as_mut().unwrap()
        };

        let source = source.next().map(|source| mmu.read(source));
        let destination = destination.next();

        Some(move |mut state: WriteOnlyState| {
            if let (Some(source), Some(destination)) = (source, destination) {
                state.write_to_oam(destination, source);
                state.set_dma_state(DmaState::Active);
            } else {
                self.0 = None;
                state.set_dma_state(DmaState::Off);
            }
        })
    }
}
