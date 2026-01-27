use std::ffi::CStr;

use gebeh::InstantRtc;
use gebeh_core::Emulator;
use gebeh_front_helper::get_mbc;

fn dmg_sound(name: &str) {
    let rom = std::fs::read(format!(
        "./downloads/gb-test-roms-master/dmg_sound/rom_singles/{name}.gb"
    ))
    .unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut machine = Emulator::default();

    while mbc.get_ram_to_save().unwrap()[0] == 0x80
        || mbc.get_ram_to_save().unwrap()[1..4] != [0xde, 0xb0, 0x61]
        || mbc.get_ram_to_save().unwrap()[4] == 0
    {
        machine.execute(mbc.as_mut());
    }

    let output = CStr::from_bytes_until_nul(&mbc.get_ram_to_save().unwrap()[4..])
        .unwrap()
        .to_str()
        .unwrap();

    assert!(output.contains("Passed"), "Received: {output}");
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
#[ignore]
fn wave_read_while_on() {
    dmg_sound("09-wave read while on");
}

#[test]
#[ignore]
fn wave_trigger_while_on() {
    dmg_sound("10-wave trigger while on");
}

#[test]
fn regs_after_power() {
    dmg_sound("11-regs after power");
}

#[test]
#[ignore]
fn wave_write_while_on() {
    dmg_sound("12-wave write while on");
}
