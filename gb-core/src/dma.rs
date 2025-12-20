use core::ops::Range;

use crate::{
    StateMachine,
    state::{ECHO_RAM, State, WORK_RAM, WriteOnlyState},
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

impl StateMachine for Dma {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        let is_requesting = state.dma_request;
        let is_active = state.is_dma_active;

        if !is_active && !is_requesting {
            return None;
        }

        // https://gbdev.io/pandocs/OAM_DMA_Transfer.html#ff46--dma-oam-dma-source-address--start
        // the mooneye emulator can set the register past 0xdf so we'll do the same
        let register = state.dma_register;

        let source = self.0.next().map(|source| {
            (
                source,
                state.mmu().read(
                    // if greater than 0xdfff then the dma has access to a bigger echo ram than the cpu
                    // from https://github.com/Gekkio/mooneye-gb/blob/3856dcbca82a7d32bd438cc92fd9693f868e2e23/core/src/hardware.rs#L215
                    if source >= ECHO_RAM {
                        source - ECHO_RAM + WORK_RAM
                    } else {
                        source
                    },
                ),
            )
        });

        // must check now or the DMA will be active one cycle longer
        let is_empty = self.0.is_empty();

        Some(move |mut state: WriteOnlyState| {
            if is_requesting {
                state.set_dma_active(true);
                state.set_dma_request(false);
                // for next cycle
                *self =
                    Self(u16::from_be_bytes([register, 0])..u16::from_be_bytes([register, 0xa0]));
            } else if is_empty {
                state.set_dma_active(false);
            }

            if let Some((address, value)) = source {
                state.write_to_oam(address as u8, value);
            }
        })
    }
}
