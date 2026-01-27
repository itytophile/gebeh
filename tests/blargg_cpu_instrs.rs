use crate::common::{TestSerial, machine_to_serial_iter};
use gebeh::InstantRtc;
use gebeh_core::Emulator;
use gebeh_front_helper::get_mbc;

mod common;

fn cpu_instrs(name: &str) {
    let expected = format!("{name}\n\n\nPassed\n");
    let len = expected.len();

    let rom = std::fs::read(format!(
        "./downloads/gb-test-roms-master/cpu_instrs/individual/{name}.gb"
    ))
    .unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut machine = Emulator::default();
    let mut serial = TestSerial(None);

    let buffer: Vec<_> = machine_to_serial_iter(&mut machine, &mut serial, mbc.as_mut())
        .take(len)
        .collect();

    assert_eq!(expected, str::from_utf8(&buffer).unwrap());
}

#[test]
fn special() {
    cpu_instrs("01-special");
}

#[test]
fn interrupts() {
    cpu_instrs("02-interrupts");
}

#[test]
fn op_sp_hl() {
    cpu_instrs("03-op sp,hl");
}

#[test]
fn op_r_imm() {
    cpu_instrs("04-op r,imm");
}

#[test]
fn op_rp() {
    cpu_instrs("05-op rp");
}

#[test]
fn ld_r_r() {
    cpu_instrs("06-ld r,r");
}

#[test]
fn jr_jp_call_ret_rst() {
    cpu_instrs("07-jr,jp,call,ret,rst");
}

#[test]
fn misc_instrs() {
    cpu_instrs("08-misc instrs");
}

#[test]
fn op_r_r() {
    cpu_instrs("09-op r,r");
}

#[test]
fn bit_ops() {
    cpu_instrs("10-bit ops");
}

#[test]
fn op_a_hl() {
    cpu_instrs("11-op a,(hl)");
}
