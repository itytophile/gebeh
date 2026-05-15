use std::ffi::CStr;

use crate::common::machine_to_serial_iter;
use arrayvec::ArrayVec;
use gebeh::InstantRtc;
use gebeh_core::{Cgb, Dmg, Emulator, EmulatorExt, Model};
use gebeh_front_helper::get_mbc;

mod common;

fn instr_timing_inner<M: Model>() {
    const EXPECTED: &str = "instr_timing\n\n\nPassed";
    const LEN: usize = EXPECTED.len();

    let rom =
        std::fs::read("./downloads/gb-test-roms-master/instr_timing/instr_timing.gb").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc(rom, InstantRtc::default()).unwrap();
    let mut machine = Emulator::<M>::default();

    let buffer: ArrayVec<u8, LEN> = machine_to_serial_iter(&mut machine, mbc.as_mut())
        .take(LEN)
        .collect();

    assert_eq!(EXPECTED, str::from_utf8(&buffer).unwrap());
}

#[test]
fn instr_timing() {
    instr_timing_inner::<Dmg>()
}

#[test]
fn instr_timing_cgb() {
    instr_timing_inner::<Cgb>()
}

fn mem_timing_inner<M: Model>() {
    const EXPECTED: &str = "mem_timing\n\n01:ok  02:ok  03:ok  \n\nPassed";
    const LEN: usize = EXPECTED.len();

    let rom = std::fs::read("./downloads/gb-test-roms-master/mem_timing/mem_timing.gb").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc(rom, InstantRtc::default()).unwrap();
    let mut machine = Emulator::<M>::default();

    let buffer: ArrayVec<u8, LEN> = machine_to_serial_iter(&mut machine, mbc.as_mut())
        .take(LEN)
        .collect();

    assert_eq!(EXPECTED, str::from_utf8(&buffer).unwrap());
}

#[test]
fn mem_timing() {
    mem_timing_inner::<Dmg>();
}

#[test]
fn mem_timing_cgb() {
    mem_timing_inner::<Cgb>();
}

fn mem_timing_2_inner<M: Model>() {
    const EXPECTED: &str = "mem_timing\n\n01:ok  02:ok  03:ok  \n\nPassed\n";
    let rom = std::fs::read("./downloads/gb-test-roms-master/mem_timing-2/mem_timing.gb").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc(rom, InstantRtc::default()).unwrap();
    // we have to clear the ram to have a 0 terminated string in ram...
    mbc.load_saved_ram(&[0; 0x8000]);
    let mut machine = Emulator::<M>::default();

    let output = loop {
        machine.execute(mbc.as_mut());
        let ram = mbc.get_ram_to_save().unwrap();
        if ram[1..4] != [0xDE, 0xB0, 0x61] || ram[0] == 0x80 {
            continue;
        }
        let output = CStr::from_bytes_until_nul(&ram[4..])
            .unwrap()
            .to_str()
            .unwrap();
        if output.len() == EXPECTED.len() {
            break output;
        }
    };

    assert_eq!(EXPECTED, output)
}

#[test]
fn mem_timing_2() {
    mem_timing_2_inner::<Dmg>();
}

#[test]
fn mem_timing_2_cgb() {
    mem_timing_2_inner::<Cgb>();
}
