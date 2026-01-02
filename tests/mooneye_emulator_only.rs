use gebeh::get_mbc;
use gebeh_core::Emulator;

fn test_mooneye(path: &str) {
    let rom = std::fs::read(format!(
        "/home/ityt/Téléchargements/mts-20240926-1737-443f6e1/emulator-only/{path}"
    ))
    .unwrap();
    let rom = rom.as_slice();
    let mut emulator = Emulator::default();
    let mut mbc = get_mbc(rom).unwrap();

    // https://github.com/Gekkio/mooneye-test-suite/tree/main?tab=readme-ov-file#passfail-reporting
    while emulator.get_cpu().current_opcode != 0x40 {
        emulator.execute(mbc.as_mut());
    }

    let cpu = emulator.get_cpu();

    assert_eq!(3, cpu.b);
    assert_eq!(5, cpu.c);
    assert_eq!(8, cpu.d);
    assert_eq!(13, cpu.e);
    assert_eq!(21, cpu.h);
    assert_eq!(34, cpu.l);
}

// mbc1

#[test]
fn mbc1_bits_bank1() {
    test_mooneye("mbc1/bits_bank1.gb");
}

#[test]
fn mbc1_bits_bank2() {
    test_mooneye("mbc1/bits_bank2.gb");
}

#[test]
fn mbc1_bits_mode() {
    test_mooneye("mbc1/bits_mode.gb");
}

#[test]
fn mbc1_bits_ramg() {
    test_mooneye("mbc1/bits_ramg.gb");
}

#[test]
fn mbc1_multicart_rom_8mb() {
    test_mooneye("mbc1/multicart_rom_8Mb.gb");
}

#[test]
fn mbc1_ram_256kb() {
    test_mooneye("mbc1/ram_256kb.gb");
}

#[test]
fn mbc1_ram_64kb() {
    test_mooneye("mbc1/ram_64kb.gb");
}

#[test]
fn mbc1_rom_16mb() {
    test_mooneye("mbc1/rom_16Mb.gb");
}

#[test]
fn mbc1_rom_1mb() {
    test_mooneye("mbc1/rom_1Mb.gb");
}

#[test]
fn mbc1_rom_2mb() {
    test_mooneye("mbc1/rom_2Mb.gb");
}

#[test]
fn mbc1_rom_4mb() {
    test_mooneye("mbc1/rom_4Mb.gb");
}

#[test]
fn mbc1_rom_512kb() {
    test_mooneye("mbc1/rom_512kb.gb");
}

#[test]
fn mbc1_rom_8mb() {
    test_mooneye("mbc1/rom_8Mb.gb");
}
