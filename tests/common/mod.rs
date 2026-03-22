use gebeh_core::{Emulator, mbc::Mbc};
use std::iter;

pub fn machine_to_serial_iter(
    emulator: &mut Emulator,
    mbc: &mut dyn Mbc,
) -> impl Iterator<Item = u8> {
    iter::from_fn(move || {
        loop {
            emulator.execute(mbc);
            if let Some(byte) = emulator.serial.get_serial_byte() {
                emulator
                    .serial
                    .set_msg_from_slave(byte, &mut emulator.state);
                return Some(byte);
            }
        }
    })
}
