use gebeh_core::{
    Emulator,
    mbc::Mbc,
    state::{SerialControl, State},
};
use std::iter;

#[derive(Clone, Default)]
pub struct TestSerial(pub Option<u8>);

impl TestSerial {
    pub fn execute(&mut self, state: &mut State) {
        // if transfer enable
        let mut must_clear = false;
        if state
            .sc
            .contains(SerialControl::TRANSFER_ENABLE | SerialControl::CLOCK_SELECT)
        {
            self.0 = Some(state.sb);
            must_clear = true;
        }
        if must_clear {
            state.sc.remove(SerialControl::TRANSFER_ENABLE);
        }
    }
}

pub fn machine_to_serial_iter(
    emulator: &mut Emulator,
    serial: &mut TestSerial,
    mbc: &mut dyn Mbc,
) -> impl Iterator<Item = u8> {
    iter::from_fn(move || {
        loop {
            emulator.execute(mbc);
            serial.execute(&mut emulator.state);
            if let Some(byte) = serial.0.take() {
                return Some(byte);
            }
        }
    })
}
