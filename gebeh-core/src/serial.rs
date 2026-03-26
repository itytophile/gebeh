use crate::{
    FallingEdge,
    state::{Interruptions, SerialControl, State},
};

#[derive(Clone)]
enum SerialControlState {
    NoTransfer { is_master: bool },
    Slave,
    Master { serial_count: u8 },
}

impl Default for SerialControlState {
    fn default() -> Self {
        SerialControlState::NoTransfer { is_master: false }
    }
}

#[derive(Clone, Default)]
pub struct Serial {
    pub sb: u8,
    sc: SerialControlState,
    falling_edge: FallingEdge,
}

impl Serial {
    pub fn set_control(&mut self, sc: SerialControl) {
        match (
            sc.contains(SerialControl::CLOCK_SELECT),
            sc.contains(SerialControl::TRANSFER_ENABLE),
        ) {
            (true, true) => {
                if !core::matches!(self.sc, SerialControlState::Master { .. }) {
                    self.sc = SerialControlState::Master { serial_count: 0 };
                }
            }
            (is_master, false) => {
                self.sc = SerialControlState::NoTransfer { is_master };
            }
            (false, true) => self.sc = SerialControlState::Slave,
        }
    }

    pub fn get_control(&self) -> SerialControl {
        match self.sc {
            SerialControlState::NoTransfer { is_master } => {
                let mut sc = SerialControl::empty();
                sc.set(SerialControl::CLOCK_SELECT, is_master);
                sc
            }
            SerialControlState::Slave => SerialControl::TRANSFER_ENABLE,
            SerialControlState::Master { .. } => {
                SerialControl::TRANSFER_ENABLE | SerialControl::CLOCK_SELECT
            }
        }
    }

    pub fn execute(&mut self, system_clock: u16) {
        // don't check that inside the SerialControlState::Master if block
        let clock_16384_hz = self.falling_edge.update(system_clock & (1 << 6) != 0);

        if clock_16384_hz
            && let SerialControlState::Master { serial_count } = &mut self.sc
            && *serial_count < READY_COUNT
        {
            *serial_count += 1;
        }
    }

    pub fn can_accept_msg_from_slave(&self) -> bool {
        core::matches!(
            self.sc,
            SerialControlState::Master {
                serial_count: READY_COUNT
            }
        )
    }

    // always check can_accept_msg_from_slave before
    pub fn set_msg_from_slave(&mut self, byte: u8, state: &mut State) -> u8 {
        let response = self.sb;
        self.sc = SerialControlState::NoTransfer { is_master: true };
        state.interrupt_flag.insert(Interruptions::SERIAL);
        self.sb = byte;
        response
    }

    pub fn set_msg_from_master(&mut self, byte: u8, state: &mut State) -> u8 {
        if !core::matches!(self.sc, SerialControlState::Slave) {
            return 0xff;
        }

        let response = self.sb;
        self.sc = SerialControlState::NoTransfer { is_master: false };
        state.interrupt_flag.insert(Interruptions::SERIAL);
        self.sb = byte;
        response
    }
}

const READY_COUNT: u8 = 16;
