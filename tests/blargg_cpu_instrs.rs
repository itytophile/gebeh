use crate::common::machine_to_serial_iter;
use gebeh::InstantRtc;
use gebeh_core::{Cgb, Dmg, Emulator, Model};
use gebeh_front_helper::get_mbc;

mod common;

fn cpu_instrs<M: Model>(name: &str) {
    let expected = format!("{name}\n\n\nPassed\n");
    let len = expected.len();

    let rom = std::fs::read(format!(
        "./downloads/gb-test-roms-master/cpu_instrs/individual/{name}.gb"
    ))
    .unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc(rom, InstantRtc::default()).unwrap();
    let mut machine = Emulator::<M>::default();

    let buffer: Vec<_> = machine_to_serial_iter(&mut machine, mbc.as_mut())
        .take(len)
        .collect();

    assert_eq!(expected, str::from_utf8(&buffer).unwrap());
}

#[test]
fn special() {
    cpu_instrs::<Dmg>("01-special");
}

#[test]
fn interrupts() {
    cpu_instrs::<Dmg>("02-interrupts");
}

#[test]
fn op_sp_hl() {
    cpu_instrs::<Dmg>("03-op sp,hl");
}

#[test]
fn op_r_imm() {
    cpu_instrs::<Dmg>("04-op r,imm");
}

#[test]
fn op_rp() {
    cpu_instrs::<Dmg>("05-op rp");
}

#[test]
fn ld_r_r() {
    cpu_instrs::<Dmg>("06-ld r,r");
}

#[test]
fn jr_jp_call_ret_rst() {
    cpu_instrs::<Dmg>("07-jr,jp,call,ret,rst");
}

#[test]
fn misc_instrs() {
    cpu_instrs::<Dmg>("08-misc instrs");
}

#[test]
fn op_r_r() {
    cpu_instrs::<Dmg>("09-op r,r");
}

#[test]
fn bit_ops() {
    cpu_instrs::<Dmg>("10-bit ops");
}

#[test]
fn op_a_hl() {
    cpu_instrs::<Dmg>("11-op a,(hl)");
}

// cgb

#[test]
fn special_cgb() {
    cpu_instrs::<Cgb>("01-special");
}

#[test]
fn interrupts_cgb() {
    cpu_instrs::<Cgb>("02-interrupts");
}

#[test]
fn op_sp_hl_cgb() {
    cpu_instrs::<Cgb>("03-op sp,hl");
}

#[test]
fn op_r_imm_cgb() {
    cpu_instrs::<Cgb>("04-op r,imm");
}

#[test]
fn op_rp_cgb() {
    cpu_instrs::<Cgb>("05-op rp");
}

#[test]
fn ld_r_r_cgb() {
    cpu_instrs::<Cgb>("06-ld r,r");
}

#[test]
fn jr_jp_call_ret_rst_cgb() {
    cpu_instrs::<Cgb>("07-jr,jp,call,ret,rst");
}

#[test]
fn misc_instrs_cgb() {
    cpu_instrs::<Cgb>("08-misc instrs");
}

#[test]
fn op_r_r_cgb() {
    cpu_instrs::<Cgb>("09-op r,r");
}

#[test]
fn bit_ops_cgb() {
    cpu_instrs::<Cgb>("10-bit ops");
}

#[test]
fn op_a_hl_cgb() {
    cpu_instrs::<Cgb>("11-op a,(hl)");
}
