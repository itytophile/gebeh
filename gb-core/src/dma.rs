use core::ops::Range;

use crate::{
    StateMachine,
    ic::Ints,
    state::{ECHO_RAM, State, WORK_RAM},
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
    fn execute(&mut self, state: &mut State, cycle_count: u64) {
        if let Some(address) = self.0.next() {
            state.is_dma_active = true;
            state.oam[usize::from(address as u8)] = state.mmu().read(
                // if greater than 0xdfff then the dma has access to a bigger echo ram than the cpu
                // from https://github.com/Gekkio/mooneye-gb/blob/3856dcbca82a7d32bd438cc92fd9693f868e2e23/core/src/hardware.rs#L215
                if address >= ECHO_RAM {
                    address - ECHO_RAM + WORK_RAM
                } else {
                    address
                },
                cycle_count,
                Ints::empty(),
            );
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
