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

    pub fn needs_message(&self) -> bool {
        if let SerialControlState::Master {
            cycles_since_enabled,
        } = self.sc
            && cycles_since_enabled > 0
        {
            true
        } else {
            false
        }
    }

    pub fn get_serial_byte(&self) -> Option<u8> {
        if let SerialControlState::Master {
            cycles_since_enabled: 0,
        } = self.sc
        {
            return Some(self.sb);
        }
        None
    }

    pub fn is_master(&self) -> bool {
        matches!(
            self.sc,
            SerialControlState::Master { .. } | SerialControlState::NoTransfer { is_master: true }
        )
    }

    fn accept_byte(&mut self, byte: u8, state: &mut State) {
        self.sc = SerialControlState::NoTransfer {
            is_master: self.is_master(),
        };
        state.interrupt_flag.insert(Interruptions::SERIAL);
        self.sb = byte;
    }

    pub fn set_msg_from_slave(&mut self, byte: u8, state: &mut State) {
        if core::matches!(self.sc, SerialControlState::Master { .. }) {
            self.accept_byte(byte, state);
        }
    }

    pub fn set_msg_from_master(&mut self, byte: u8, state: &mut State) -> u8 {
        if let SerialControlState::Slave = self.sc {
            let response = self.sb;
            self.accept_byte(byte, state);
            return response;
        }

        0xff
    }
}
