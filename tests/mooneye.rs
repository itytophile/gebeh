use gb_core::{
    StateMachine,
    cpu::Cpu,
    dma::Dma,
    ppu::{Ppu, Speeder},
    state::{State, WriteOnlyState},
    timer::Timer,
};
use std::num::NonZeroU8;

fn test_mooneye(path: &str) {
    let rom = std::fs::read(format!(
        "/home/ityt/Téléchargements/mts-20240926-1737-443f6e1/acceptance/{path}"
    ))
    .unwrap();
    let mut state = State::new(rom.leak());
    let mut machine = Dma::default()
        .compose(Speeder(Ppu::default(), NonZeroU8::new(4).unwrap()))
        .compose(Cpu::default())
        .compose(Timer);

    // https://github.com/Gekkio/mooneye-test-suite/tree/main?tab=readme-ov-file#passfail-reporting
    while machine.0.1.current_opcode != 0x40 {
        log::warn!("clock");
        machine.execute(&state).unwrap()(WriteOnlyState::new(&mut state));
    }

    let ((_, cpu), _) = machine;

    assert_eq!(3, cpu.b);
    assert_eq!(5, cpu.c);
    assert_eq!(8, cpu.d);
    assert_eq!(13, cpu.e);
    assert_eq!(21, cpu.h);
    assert_eq!(34, cpu.l);
}

#[test]
fn add_sp_e_timing() {
    test_mooneye("add_sp_e_timing.gb")
}

#[test]
fn mem_oam() {
    test_mooneye("bits/mem_oam.gb")
}

#[test]
fn reg_f() {
    test_mooneye("bits/reg_f.gb")
}

#[test]
fn unused_hwio_gs() {
    test_mooneye("bits/unused_hwio-GS.gb")
}

#[test]
fn call_cc_timing() {
    test_mooneye("call_cc_timing.gb");
}

#[test]
fn call_cc_timing2() {
    test_mooneye("call_cc_timing2.gb");
}

#[test]
fn call_timing() {
    test_mooneye("call_timing.gb");
}

#[test]
fn call_timing2() {
    test_mooneye("call_timing2.gb");
}

#[test]
fn di_timing_gs() {
    test_mooneye("di_timing-GS.gb");
}

#[test]
fn div_timing() {
    test_mooneye("div_timing.gb");
}

#[test]
fn ei_sequence() {
    test_mooneye("ei_sequence.gb");
}

#[test]
fn ei_timing() {
    test_mooneye("ei_timing.gb");
}

#[test]
fn halt_ime0_ei() {
    test_mooneye("halt_ime0_ei.gb");
}

#[test]
fn halt_ime0_nointr_timing() {
    test_mooneye("halt_ime0_nointr_timing.gb");
}

#[test]
fn halt_ime1_timing() {
    test_mooneye("halt_ime1_timing.gb");
}

#[test]
fn halt_ime1_timing2_gs() {
    test_mooneye("halt_ime1_timing2-GS.gb");
}

#[test]
fn if_ie_registers() {
    test_mooneye("if_ie_registers.gb");
}

#[test]
fn intr_timing() {
    test_mooneye("intr_timing.gb");
}

#[test]
fn jp_cc_timing() {
    test_mooneye("jp_cc_timing.gb");
}

#[test]
fn jp_timing() {
    test_mooneye("jp_timing.gb");
}
