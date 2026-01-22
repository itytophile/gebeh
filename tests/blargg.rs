use std::ffi::CStr;

use crate::common::{TestSerial, machine_to_serial_iter};
use arrayvec::ArrayVec;
use gebeh::InstantRtc;
use gebeh_core::Emulator;
use gebeh_front_helper::get_mbc;

mod common;

#[test]
fn cpu_instrs() {
    const EXPECTED: &str = "cpu_instrs\n\n01:ok  02:ok  03:ok  04:ok  05:ok  06:ok  07:ok  08:ok  09:ok  10:ok  11:ok  \n\nPassed all tests";
    const LEN: usize = EXPECTED.len();

    let rom = std::fs::read("./downloads/gb-test-roms-master/cpu_instrs/cpu_instrs.gb").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut machine = Emulator::default();
    let mut serial = TestSerial(None);

    let buffer: ArrayVec<u8, LEN> = machine_to_serial_iter(&mut machine, &mut serial, mbc.as_mut())
        .take(LEN)
        .collect();

    assert_eq!(EXPECTED, str::from_utf8(&buffer).unwrap());
}

#[test]
fn instr_timing() {
    const EXPECTED: &str = "instr_timing\n\n\nPassed";
    const LEN: usize = EXPECTED.len();

    let rom =
        std::fs::read("./downloads/gb-test-roms-master/instr_timing/instr_timing.gb").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut machine = Emulator::default();
    let mut serial = TestSerial(None);

    let buffer: ArrayVec<u8, LEN> = machine_to_serial_iter(&mut machine, &mut serial, mbc.as_mut())
        .take(LEN)
        .collect();

    assert_eq!(EXPECTED, str::from_utf8(&buffer).unwrap());
}

#[test]
fn mem_timing() {
    const EXPECTED: &str = "mem_timing\n\n01:ok  02:ok  03:ok  \n\nPassed";
    const LEN: usize = EXPECTED.len();

    let rom = std::fs::read("./downloads/gb-test-roms-master/mem_timing/mem_timing.gb").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut machine = Emulator::default();
    let mut serial = TestSerial(None);

    let buffer: ArrayVec<u8, LEN> = machine_to_serial_iter(&mut machine, &mut serial, mbc.as_mut())
        .take(LEN)
        .collect();

    assert_eq!(EXPECTED, str::from_utf8(&buffer).unwrap());
}

#[test]
fn mem_timing_2() {
    const EXPECTED: &str = "mem_timing\n\n01:ok  02:ok  03:ok  \n\nPassed\n";
    let rom = std::fs::read("./downloads/gb-test-roms-master/mem_timing-2/mem_timing.gb").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut machine = Emulator::default();

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
