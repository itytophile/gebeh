use std::{fs::File, io::BufReader};

use gebeh::InstantRtc;
use gebeh_core::{
    Cgb, Dmg, Emulator, EmulatorExt, HEIGHT, WIDTH,
    ppu::{color::DmgColor, scanline::Scanline},
};
use gebeh_front_helper::get_mbc;

#[test]
fn dmg_acid2() {
    let rom = std::fs::read("./downloads/dmg-acid2.gb").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc(rom, InstantRtc::default()).unwrap();
    let mut emulator = Emulator::<Dmg>::default();
    let mut previous_ly = None;
    let expected = include_bytes!("acid2_expected.txt");
    let split = expected.split(|a| *a == b'\n').map(|slice| {
        slice.iter().map(|c| match c {
            b'.' => DmgColor::White,
            b'-' => DmgColor::LightGray,
            b'#' => DmgColor::DarkGray,
            b'@' => DmgColor::Black,
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

#[test]
fn cgb_acid2() {
    let decoder = png::Decoder::new(BufReader::new(
        File::open("./tests/cgb_acid2_expected.png").unwrap(),
    ));
    let mut reader = decoder.read_info().unwrap();
    let palette = reader.info().palette.as_ref().unwrap().clone().into_owned();
    let palette = palette.as_chunks::<3>().0;
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut buf).unwrap();
    let buf: Vec<u16> = buf
        .into_iter()
        .flat_map(|palette_index_2| [palette_index_2 >> 4, palette_index_2 & 0x0f])
        .map(|palette_index| {
            let [r, g, b] = palette[usize::from(palette_index)];
            let r = u16::from(r >> 3);
            let g = u16::from(g >> 3);
            let b = u16::from(b >> 3);
            r | (g << 5) | (b << 10)
        })
        .collect();
    let rom = std::fs::read("./downloads/cgb-acid2.gbc").unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc(rom, InstantRtc::default()).unwrap();
    let mut emulator = Emulator::<Cgb>::default();
    let mut previous_ly = None;
    let mut current_frame = [0u16; WIDTH as usize * HEIGHT as usize];

    loop {
        emulator.execute(mbc.as_mut());
        if let Some(scanline) = emulator.get_ppu().get_scanline_if_ready()
            && previous_ly != Some(emulator.get_ppu().get_ly())
        {
            previous_ly = Some(emulator.get_ppu().get_ly());
            current_frame[usize::from(emulator.get_ppu().get_ly()) * usize::from(WIDTH)
                ..usize::from(emulator.get_ppu().get_ly() + 1) * usize::from(WIDTH)]
                .copy_from_slice(scanline.raw());

            if emulator.get_ppu().get_ly() == HEIGHT - 1 && current_frame == buf.as_slice() {
                break;
            }
        }
    }
}
