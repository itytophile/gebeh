use crate::state::{Interruptions, State};

// There is a system counter which is 14 bits wide
// The div register is the height most significant bits of this system counter
// the system counter is incremented every m-cycle
// i.e the div counter is incremented every m-cycle * 2^6
// m-cycle frequency = 4.194304 MHz / 4
// so div frequency = 4.194304 MHz / 4 / 2^6 = 16384 Hz as pandocs says
// https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff04--div-divider-register

#[derive(Clone, Default)]
pub struct Timer {
    falling_edge_detector: bool,
}

impl Timer {
    pub fn execute(&mut self, state: &mut State, cycles: u64) {
        // we only check a single bit to see if it's a multiple of the frequency
        let frequency_mask = match state.tac & 0b11 {
            0b00 => 0x80, // multiple of 256
            0b01 => 0x02, // multiple of 4
            0b10 => 0x08, // multiple of 16
            0b11 => 0x20, // multiple of 64
            _ => unreachable!(),
        };

        state.system_counter = state.system_counter.wrapping_add(1);

        // the following checks are broken by design
        // https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#relation-between-timer-and-divider-register

        let and = state.tac & 0b100 != 0 && state.system_counter & frequency_mask != 0;

        if and == self.falling_edge_detector {
            return;
        }

        self.falling_edge_detector = and;

        if self.falling_edge_detector {
            return;
        }

        state.tima = state.tima.wrapping_add(1);

        if state.tima == 0 {
            if cycles <= 1829858 {
                log::warn!("{cycles}: overflow!");
            }
            // indeed, it's not delayed. Remove the delay fixes a mooneye test.
            // I'll investigate later (or never)
            state.interrupt_flag.insert(Interruptions::TIMER);
            state.tma_to_tima_delay = true;
        }
    }
}

// to emulate the one M-cycle delay between an overflow and tima register getting the new value
// https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#timer-overflow-behavior
// And to emulate the cpu write that cancels the overflow on the same cycle.
pub fn commit_tima_overflow(state: &mut State) {
    state.has_tima_just_overflowed = false;
    if !state.tma_to_tima_delay {
        return;
    }
    state.tma_to_tima_delay = false;
    state.tima = state.tma;
    state.has_tima_just_overflowed = true;
}
