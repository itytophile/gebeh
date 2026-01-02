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
