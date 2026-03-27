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

#[derive(Clone)]
pub struct Serial {
    pub sb: u8,
    sc: SerialControlState,
    falling_edge: FallingEdge,
    pub slave_byte: u8,
}

impl Default for Serial {
    fn default() -> Self {
        Self {
            sb: Default::default(),
            sc: Default::default(),
            falling_edge: Default::default(),
            slave_byte: 0xff,
        }
    }
}

fn get_clock_16384_hz(falling_edge: &mut FallingEdge, system_clock: u16) -> bool {
    falling_edge.update(system_clock & (1 << 5) != 0)
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

    pub fn will_emit_byte(&self, next_system_clock: u16) -> bool {
        if get_clock_16384_hz(&mut self.falling_edge.clone(), next_system_clock)
            && let SerialControlState::Master { serial_count } = self.sc
            && serial_count == READY_COUNT - 1
        {
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn execute(&mut self, system_clock: u16, state: &mut State, _: u64) -> Option<u8> {
        if get_clock_16384_hz(&mut self.falling_edge, system_clock)
            && let SerialControlState::Master { serial_count } = &mut self.sc
            && *serial_count < READY_COUNT
        {
            *serial_count += 1;

            if *serial_count == READY_COUNT {
                let response = self.sb;
                self.sc = SerialControlState::NoTransfer { is_master: true };
                state.interrupt_flag.insert(Interruptions::SERIAL);
                self.sb = self.slave_byte;
                return Some(response);
            }
        }

        None
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
