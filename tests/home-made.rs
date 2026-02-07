use std::{fs::File, io::BufReader};

use gebeh::InstantRtc;
use gebeh_core::{Emulator, HEIGHT, WIDTH};
use gebeh_front_helper::get_mbc;

fn home_made(name: &str) {
    let decoder = png::Decoder::new(BufReader::new(
        File::open(format!("./gebeh-test-roms/expected/{name}.png")).unwrap(),
    ));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut buf).unwrap();
    let rom = std::fs::read(format!("./gebeh-test-roms/{name}.gb")).unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut emulator = Emulator::default();
    let mut previous_ly = None;
    let mut current_frame = [0u8; WIDTH as usize * HEIGHT as usize / 4];
    // let path = Path::new(r"prout.png");
    // let mut file = File::create(path).unwrap();

    loop {
        emulator.execute(mbc.as_mut());
        if let Some(scanline) = emulator.get_ppu().get_scanline_if_ready()
            && previous_ly != Some(emulator.get_ppu().get_ly())
        {
            previous_ly = Some(emulator.get_ppu().get_ly());
            current_frame[usize::from(emulator.get_ppu().get_ly()) * usize::from(WIDTH) / 4
                ..usize::from(emulator.get_ppu().get_ly() + 1) * usize::from(WIDTH) / 4]
                .copy_from_slice(scanline.raw());

            // if emulator.get_ppu().get_ly() == HEIGHT - 1 {
            //     file.set_len(0).unwrap();
            //     file.seek(SeekFrom::Start(0)).unwrap();
            //     let w = &mut BufWriter::new(&mut file);
            //     let mut encoder = png::Encoder::new(w, WIDTH.into(), HEIGHT.into());
            //     encoder.set_color(png::ColorType::Grayscale);
            //     encoder.set_depth(png::BitDepth::Two);
            //     let mut writer = encoder.write_header().unwrap();

            //     writer.write_image_data(&current_frame).unwrap(); // Save
            // }

            if emulator.get_ppu().get_ly() == HEIGHT - 1 && current_frame == buf.as_slice() {
                break;
            }
        }
    }
}

#[test]
fn stat_mode_2_palette_screen_edges() {
    home_made("stat_mode_2_palette_screen_edges");
}
