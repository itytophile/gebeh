use gebeh_core::{Emulator, mbc::Mbc};
use std::iter;

pub fn machine_to_serial_iter(
    emulator: &mut Emulator,
    mbc: &mut dyn Mbc,
) -> impl Iterator<Item = u8> {
    iter::from_fn(move || {
        loop {
            if let Some(byte) = emulator.execute(mbc) {
                return Some(byte);
            }
        }
    })
}
