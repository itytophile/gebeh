use gebeh::InstantRtc;
use gebeh_core::{Emulator, HEIGHT, ppu::color::Color};
use gebeh_front_helper::get_mbc;

#[test]
fn dmg_acid2() {
    let rom = std::fs::read("./downloads/dmg-acid2.gb").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut emulator = Emulator::default();
    let mut previous_ly = None;
    let expected = include_bytes!("acid2_expected.txt");
    let split = expected.split(|a| *a == b'\n').map(|slice| {
        slice.iter().map(|c| match c {
            b'.' => Color::White,
            b'-' => Color::LightGray,
            b'#' => Color::DarkGray,
            b'@' => Color::Black,
            _ => panic!(),
        })
    });
    let mut working_split = split.clone();
    let mut all_good = true;
    loop {
        emulator.execute(mbc.as_mut());
        if let Some(scanline) = emulator.get_ppu().get_scanline_if_ready()
            && previous_ly != Some(emulator.get_ppu().get_ly())
        {
            previous_ly = Some(emulator.get_ppu().get_ly());
            all_good &= working_split.next().unwrap().eq(scanline.iter_colors());
            if emulator.get_ppu().get_ly() == HEIGHT - 1 {
                if all_good {
                    return;
                }
                working_split = split.clone();
                all_good = true;
            }
        }
    }
}
