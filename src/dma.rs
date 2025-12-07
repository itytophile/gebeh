use std::ops::{RangeFrom, RangeInclusive};

use crate::{
    StateMachine,
    state::{State, WriteOnlyState},
};

pub struct Dma(Option<(RangeInclusive<u16>, RangeFrom<u8>)>);

impl StateMachine for Dma {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        // https://gbdev.io/pandocs/OAM_DMA_Transfer.html#ff46--dma-oam-dma-source-address--start
        let mmu = state.mmu();
        let register = state.dma_register.min(0xdf);
        let (source, destination) = self.0.get_or_insert((
            u16::from_be_bytes([register, 0])..=u16::from_be_bytes([register, 0x9f]),
            0..,
        ));

        let (Some(source), Some(destination)) = (source.next(), destination.next()) else {
            return None;
        };

        let value = mmu.read(source);

        Some(move |mut state: WriteOnlyState| {
            state.write_to_oam(destination, value);
        })
    }
}
