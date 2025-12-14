use gb_core::{
    StateMachine,
    cpu::Cpu,
    dma::Dma,
    ppu::{Ppu, Speeder},
    state::{State, WriteOnlyState},
    timer::Timer,
};
use std::num::NonZeroU8;

mod common;

fn test_mooneye(path: &str) {
    let rom = std::fs::read(path).unwrap();
    let mut state = State::new(rom.leak());
    // the machine should not be affected by the composition order
    let mut machine = Dma::default()
        .compose(Speeder(Ppu::default(), NonZeroU8::new(4).unwrap()))
        .compose(Timer::default())
        .compose(Cpu::default());

    // https://github.com/Gekkio/mooneye-test-suite/tree/main?tab=readme-ov-file#passfail-reporting
    while machine.1.current_opcode != 0x40 {
        machine.execute(&state).unwrap()(WriteOnlyState::new(&mut state));
    }

    let (_, cpu) = machine;

    assert_eq!(3, cpu.b);
    assert_eq!(5, cpu.c);
    assert_eq!(8, cpu.d);
    assert_eq!(13, cpu.e);
    assert_eq!(21, cpu.h);
    assert_eq!(34, cpu.l);
}

#[test]
fn add_sp_e_timing() {
    test_mooneye(
        "/home/ityt/Téléchargements/mts-20240926-1737-443f6e1/acceptance/add_sp_e_timing.gb",
    )
}

#[test]
fn mem_oam() {
    test_mooneye("/home/ityt/Téléchargements/mts-20240926-1737-443f6e1/acceptance/bits/mem_oam.gb")
}
