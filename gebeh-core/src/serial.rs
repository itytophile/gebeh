use crate::{FallingEdge, interrupts::Interrupts};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq)]
    pub struct SerialControl: u8 {
        const TRANSFER_ENABLE = 1 << 7;
        const CLOCK_SPEED = 1 << 2;
        const CLOCK_SELECT = 1;
    }
}

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
struct SerialState {
    pub sb: u8,
    sc: SerialControlState,
    falling_edge: FallingEdge,
    pub slave_byte: u8,
    delay_int: bool,
}

#[derive(Clone, Default)]
pub struct DmgSerial(SerialState);

impl Default for SerialState {
    fn default() -> Self {
        Self {
            sb: Default::default(),
            sc: Default::default(),
            falling_edge: Default::default(),
            slave_byte: 0xff,
            delay_int: false,
        }
    }
}

fn get_clock_16384_hz(falling_edge: &mut FallingEdge, system_clock: u16) -> bool {
    falling_edge.update(system_clock & (1 << 5) != 0)
}

fn get_clock_32768_hz(falling_edge: &mut FallingEdge, system_clock: u16) -> bool {
    falling_edge.update(system_clock & (1 << 4) != 0)
}

const READY_COUNT: u8 = 16;

pub trait Serial: Clone + Send + Sync {
    fn write_sc(&mut self, sc: SerialControl);
    fn read_sc(&self) -> u8;
    fn write_sb(&mut self, value: u8);
    fn read_sb(&self) -> u8;
    fn will_emit_byte(&self, next_system_clock: u16) -> bool;
    #[must_use]
    fn execute(&mut self, system_clock: u16, interrupts: &mut Interrupts, _: u64) -> Option<u8>;
    fn set_msg_from_master(&mut self, byte: u8, interrupts: &mut Interrupts) -> u8;
    fn set_slave_byte(&mut self, value: u8);
    fn get_slave_byte(&self) -> u8;
}

impl Serial for DmgSerial {
    fn write_sc(&mut self, sc: SerialControl) {
        match (
            sc.contains(SerialControl::CLOCK_SELECT),
            sc.contains(SerialControl::TRANSFER_ENABLE),
        ) {
            (true, true) => {
                if !core::matches!(self.0.sc, SerialControlState::Master { .. }) {
                    self.0.sc = SerialControlState::Master { serial_count: 0 };
                }
            }
            (is_master, false) => {
                self.0.sc = SerialControlState::NoTransfer { is_master };
            }
            (false, true) => self.0.sc = SerialControlState::Slave,
        }
    }

    fn write_sb(&mut self, value: u8) {
        self.0.sb = value;
    }

    fn read_sc(&self) -> u8 {
        match self.0.sc {
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
        .bits()
            | 0b01111110
    }

    fn read_sb(&self) -> u8 {
        self.0.sb
    }

    fn will_emit_byte(&self, next_system_clock: u16) -> bool {
        if get_clock_16384_hz(&mut self.0.falling_edge.clone(), next_system_clock)
            && let SerialControlState::Master { serial_count } = self.0.sc
            && serial_count == READY_COUNT - 1
        {
            true
        } else {
            false
        }
    }

    fn execute(&mut self, system_clock: u16, interrupts: &mut Interrupts, _: u64) -> Option<u8> {
        if self.0.delay_int {
            interrupts.insert(Interrupts::SERIAL);
            self.0.delay_int = false;
        }
        if get_clock_16384_hz(&mut self.0.falling_edge, system_clock)
            && let SerialControlState::Master { serial_count } = &mut self.0.sc
            && *serial_count < READY_COUNT
        {
            *serial_count += 1;

            if *serial_count == READY_COUNT {
                let response = self.0.sb;
                self.0.sc = SerialControlState::NoTransfer { is_master: true };
                self.0.delay_int = true;
                self.0.sb = self.0.slave_byte;
                return Some(response);
            }
        }

        None
    }

    fn set_msg_from_master(&mut self, byte: u8, interrupts: &mut Interrupts) -> u8 {
        if !core::matches!(self.0.sc, SerialControlState::Slave) {
            return 0xff;
        }

        let response = self.0.sb;
        self.0.sc = SerialControlState::NoTransfer { is_master: false };
        interrupts.insert(Interrupts::SERIAL);
        self.0.sb = byte;
        response
    }

    fn set_slave_byte(&mut self, value: u8) {
        self.0.slave_byte = value;
    }

    fn get_slave_byte(&self) -> u8 {
        self.0.slave_byte
    }
}

#[derive(Clone, Default)]
pub struct CgbSerial {
    is_double_speed: bool,
    state: SerialState,
}

impl Serial for CgbSerial {
    fn write_sc(&mut self, sc: SerialControl) {
        match (
            sc.contains(SerialControl::CLOCK_SELECT),
            sc.contains(SerialControl::TRANSFER_ENABLE),
        ) {
            (true, true) => {
                if !core::matches!(self.state.sc, SerialControlState::Master { .. }) {
                    self.state.sc = SerialControlState::Master { serial_count: 0 };
                }
            }
            (is_master, false) => {
                self.state.sc = SerialControlState::NoTransfer { is_master };
            }
            (false, true) => self.state.sc = SerialControlState::Slave,
        }
        self.is_double_speed = sc.contains(SerialControl::CLOCK_SPEED);
    }

    fn write_sb(&mut self, value: u8) {
        self.state.sb = value;
    }

    fn read_sc(&self) -> u8 {
        let mut sc = match self.state.sc {
            SerialControlState::NoTransfer { is_master } => {
                let mut sc = SerialControl::empty();
                sc.set(SerialControl::CLOCK_SELECT, is_master);
                sc
            }
            SerialControlState::Slave => SerialControl::TRANSFER_ENABLE,
            SerialControlState::Master { .. } => {
                SerialControl::TRANSFER_ENABLE | SerialControl::CLOCK_SELECT
            }
        };
        sc.set(SerialControl::CLOCK_SPEED, self.is_double_speed);
        sc.bits() | 0b0111_1100
    }

    fn read_sb(&self) -> u8 {
        self.state.sb
    }

    fn will_emit_byte(&self, next_system_clock: u16) -> bool {
        if get_clock(
            self.is_double_speed,
            &mut self.state.falling_edge.clone(),
            next_system_clock,
        ) && let SerialControlState::Master { serial_count } = self.state.sc
            && serial_count == READY_COUNT - 1
        {
            true
        } else {
            false
        }
    }

    fn execute(&mut self, system_clock: u16, interrupts: &mut Interrupts, _: u64) -> Option<u8> {
        if self.state.delay_int {
            interrupts.insert(Interrupts::SERIAL);
            self.state.delay_int = false;
        }
        if get_clock(
            self.is_double_speed,
            &mut self.state.falling_edge,
            system_clock,
        ) && let SerialControlState::Master { serial_count } = &mut self.state.sc
            && *serial_count < READY_COUNT
        {
            *serial_count += 1;

            if *serial_count == READY_COUNT {
                let response = self.state.sb;
                self.state.sc = SerialControlState::NoTransfer { is_master: true };
                self.state.delay_int = true;
                self.state.sb = self.state.slave_byte;
                return Some(response);
            }
        }

        None
    }

    fn set_msg_from_master(&mut self, byte: u8, interrupts: &mut Interrupts) -> u8 {
        if !core::matches!(self.state.sc, SerialControlState::Slave) {
            return 0xff;
        }

        let response = self.state.sb;
        self.state.sc = SerialControlState::NoTransfer { is_master: false };
        interrupts.insert(Interrupts::SERIAL);
        self.state.sb = byte;
        response
    }

    fn set_slave_byte(&mut self, value: u8) {
        self.state.slave_byte = value;
    }

    fn get_slave_byte(&self) -> u8 {
        self.state.slave_byte
    }
}

fn get_clock(is_double_speed: bool, falling_edge: &mut FallingEdge, system_clock: u16) -> bool {
    is_double_speed && get_clock_32768_hz(falling_edge, system_clock)
        || !is_double_speed && get_clock_16384_hz(falling_edge, system_clock)
}
