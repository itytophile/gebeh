use crate::state::{Interruptions, SerialControl, State};

#[derive(Clone)]
enum SerialControlState {
    NoTransfer { is_master: bool },
    Slave,
    Master { cycles_since_enabled: u16 },
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
}

impl Serial {
    pub fn set_control(&mut self, sc: SerialControl) {
        match (
            sc.contains(SerialControl::CLOCK_SELECT),
            sc.contains(SerialControl::TRANSFER_ENABLE),
        ) {
            (true, true) => {
                if !core::matches!(self.sc, SerialControlState::Master { .. }) {
                    self.sc = SerialControlState::Master {
                        cycles_since_enabled: 0,
                    };
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

    pub fn execute(&mut self) {
        if let SerialControlState::Master {
            cycles_since_enabled,
        } = &mut self.sc
        {
            *cycles_since_enabled = cycles_since_enabled.saturating_add(1);
        }
    }

    pub fn can_accept_msg_from_slave(&self) -> bool {
        core::matches!(
            self.sc,
            SerialControlState::Master {
                cycles_since_enabled: BYTE_READY_CYCLE
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

// https://gbdev.io/pandocs/Specifications.html https://gbdev.io/pandocs/Serial_Data_Transfer_(Link_Cable).html
// The system clock (4194304 / 4) divided by byte transfer frequency (8192 / 8)
// 4194304 / 4 / 8192 * 8
const BYTE_READY_CYCLE: u16 = 1024;
