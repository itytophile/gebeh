use gb_core::{
    StateMachine,
    state::{SerialControl, State, WriteOnlyState},
};
use std::iter;

#[derive(Clone)]
pub struct TestSerial(pub Option<u8>);

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

pub fn machine_to_serial_iter(
    machine: &mut (impl StateMachine, TestSerial),
    state: &mut State,
) -> impl Iterator<Item = u8> {
    iter::from_fn(move || {
        loop {
            machine.execute(state).unwrap()(WriteOnlyState::new(state));
            let (_, TestSerial(byte)) = machine;
            if let Some(byte) = byte.take() {
                return Some(byte);
            }
        }
    })
}
