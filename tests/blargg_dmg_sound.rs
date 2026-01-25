use crate::common::{TestSerial, machine_to_serial_iter};
use gebeh::InstantRtc;
use gebeh_core::Emulator;
use gebeh_front_helper::get_mbc;

mod common;

fn dmg_sound(name: &str) {
    let expected = format!("{name}\n");
    let len = expected.len();

    let rom = std::fs::read(format!(
        "./downloads/gb-test-roms-master/dmg_sound/rom_singles/{name}.gb"
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
fn registers() {
    dmg_sound("01-registers");
}

#[test]
fn len_ctr() {
    dmg_sound("02-len ctr");
}

#[test]
fn trigger() {
    dmg_sound("03-trigger");
}

#[test]
fn sweep() {
    dmg_sound("04-sweep");
}

#[test]
fn sweep_details() {
    dmg_sound("05-sweep details");
}

#[test]
fn overflow_on_trigger() {
    dmg_sound("06-overflow on trigger");
}

#[test]
fn len_sweep_period_sync() {
    dmg_sound("07-len sweep period sync");
}

#[test]
fn len_ctr_during_power() {
    dmg_sound("08-len ctr during power");
}

#[test]
fn wave_read_while_on() {
    dmg_sound("09-wave read while on");
}

#[test]
fn wave_trigger_while_on() {
    dmg_sound("10-wave trigger while on");
}

#[test]
fn regs_after_power() {
    dmg_sound("11-regs after power");
}

#[test]
fn wave_write_while_on() {
    dmg_sound("12-wave write while on");
}
