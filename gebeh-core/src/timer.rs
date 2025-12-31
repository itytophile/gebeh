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
    tma: u8,
    tac: u8,
    tima: u8,
    tma_to_tima_delay: bool,
    has_tima_just_overflowed: bool, // will block write in the cycle after the overflow
    system_counter: u16,
}

impl Timer {
    pub fn get_div(&self) -> u8 {
        (self.system_counter >> 6) as u8
    }
    pub fn reset_system_counter(&mut self) {
        self.system_counter = 0;
    }
    pub fn set_tma(&mut self, value: u8) {
        if self.has_tima_just_overflowed {
            // should not conflict with a timer increment hopefully
            self.tima = value;
        }
        self.tma = value
    }
    pub fn get_tma(&self) -> u8 {
        self.tma
    }
    pub fn set_tac(&mut self, value: u8) {
        self.tac = value;
    }
    pub fn get_tima(&self) -> u8 {
        self.tima
    }
    pub fn get_tac(&self) -> u8 {
        self.tac | 0b11111000
    }
    pub fn set_tima(&mut self, value: u8) {
        if self.has_tima_just_overflowed {
            return;
        }
        // Cancel tima overflow if it's in the same cycle
        // https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#timer-overflow-behavior
        self.tima = value;
        self.tma_to_tima_delay = false;
    }
    pub fn execute(&mut self, state: &mut State, cycles: u64) {
        // we only check a single bit to see if it's a multiple of the frequency
        let frequency_mask = match self.tac & 0b11 {
            0b00 => 0x80, // multiple of 256
            0b01 => 0x02, // multiple of 4
            0b10 => 0x08, // multiple of 16
            0b11 => 0x20, // multiple of 64
            _ => unreachable!(),
        };

        self.system_counter = self.system_counter.wrapping_add(1);

        // the following checks are broken by design
        // https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#relation-between-timer-and-divider-register

        let and = self.tac & 0b100 != 0 && self.system_counter & frequency_mask != 0;

        if and == self.falling_edge_detector {
            return;
        }

        self.falling_edge_detector = and;

        if self.falling_edge_detector {
            return;
        }

        self.tima = self.tima.wrapping_add(1);

        if self.tima == 0 {
            if cycles <= 1829858 {
                log::warn!("{cycles}: overflow!");
            }
            // indeed, it's not delayed. Remove the delay fixes a mooneye test.
            // I'll investigate later (or never)
            state.interrupt_flag.insert(Interruptions::TIMER);
            self.tma_to_tima_delay = true;
        }
    }

    // to emulate the one M-cycle delay between an overflow and tima register getting the new value
    // https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#timer-overflow-behavior
    // And to emulate the cpu write that cancels the overflow on the same cycle.
    pub fn commit_tima_overflow(&mut self) {
        self.has_tima_just_overflowed = false;
        if !self.tma_to_tima_delay {
            return;
        }
        self.tma_to_tima_delay = false;
        self.tima = self.tma;
        self.has_tima_just_overflowed = true;
    }
}
