use crate::common::{TestSerial, machine_to_serial_iter};
use arrayvec::ArrayVec;
use gb_core::{
    StateMachine,
    cpu::Cpu,
    ppu::{Ppu, Speeder},
    state::State,
    timer::Timer,
};
use std::num::NonZeroU8;

mod common;

#[test]
fn cpu_instrs() {
    const EXPECTED: &str = "cpu_instrs\n\n01:ok  02:ok  03:ok  04:ok  05:ok  06:ok  07:ok  08:ok  09:ok  10:ok  11:ok  \n\nPassed all tests";
    const LEN: usize = EXPECTED.len();

    let rom =
        std::fs::read("/home/ityt/Documents/git/gb-test-roms/cpu_instrs/cpu_instrs.gb").unwrap();
    let mut state = State::new(rom.leak());
    // the machine should not be affected by the composition order
    let mut machine = Cpu::default()
        .compose(Timer::default())
        .compose(Speeder(Ppu::default(), NonZeroU8::new(4).unwrap()))
        .compose(TestSerial(None));

    let buffer: ArrayVec<u8, LEN> = machine_to_serial_iter(&mut machine, &mut state)
        .take(LEN)
        .collect();

    assert_eq!(EXPECTED, str::from_utf8(&buffer).unwrap());
}
