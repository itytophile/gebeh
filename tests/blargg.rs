use std::{iter, num::NonZeroU8};

use arrayvec::ArrayVec;
use testouille_emulator_future::{
    StateMachine,
    cpu::Cpu,
    ppu::{Ppu, Speeder},
    state::{SerialControl, State, WriteOnlyState},
    timer::Timer,
};

struct TestSerial(Option<u8>);

impl StateMachine for TestSerial {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        // if transfer enable
        let mut must_clear = false;
        if state
            .sc
            .contains(SerialControl::TRANSFER_ENABLE | SerialControl::CLOCK_SELECT)
        {
            self.0 = Some(state.sb);
            must_clear = true;
        }
        Some(move |mut state: WriteOnlyState| {
            if must_clear {
                state.get_sc_mut().remove(SerialControl::TRANSFER_ENABLE);
            }
        })
    }
}

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

    let buffer: ArrayVec<u8, LEN> = iter::from_fn(|| {
        loop {
            machine.execute(&state).unwrap()(WriteOnlyState::new(&mut state));
            let (_, TestSerial(byte)) = &mut machine;
            if let Some(byte) = byte.take() {
                return Some(byte);
            }
        }
    })
    .take(LEN)
    .collect();

    assert_eq!(EXPECTED, str::from_utf8(&buffer).unwrap());
}
