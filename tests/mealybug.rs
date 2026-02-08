use std::{fs::File, io::BufReader};

use gebeh::InstantRtc;
use gebeh_core::{Emulator, HEIGHT, WIDTH};
use gebeh_front_helper::get_mbc;
use png::BitDepth;

fn mealybug(name: &str) {
    mealybug_inner(
        &format!("downloads/mealybug-tearoom-tests-master/roms/{name}.gb"),
        &format!("downloads/mealybug-tearoom-tests-master/expected/DMG-blob/{name}.png"),
    )
}

fn two_bits_depth_byte_to_one_bit_depth_nimble(a: u8) -> u8 {
    (a & 0b11 != 0) as u8
        | (((a & 0b1100 != 0) as u8) << 1)
        | (((a & 0b110000 != 0) as u8) << 2)
        | (((a & 0b11000000 != 0) as u8) << 3)
}

fn two_bits_depth_to_one_bit_depth_image(two_bits: &[u8]) -> Vec<u8> {
    two_bits
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(|[a, b]| {
            (two_bits_depth_byte_to_one_bit_depth_nimble(a) << 4)
                | two_bits_depth_byte_to_one_bit_depth_nimble(b)
        })
        .collect()
}

fn mealybug_inner(rom: &str, expected: &str) {
    let decoder = png::Decoder::new(BufReader::new(File::open(expected).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let (_, depth) = reader.output_color_type();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut buf).unwrap();
    let rom = std::fs::read(rom).unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut emulator = Emulator::default();
    let mut previous_ly = None;
    let mut current_frame = [0u8; WIDTH as usize * HEIGHT as usize / 4];
    // let mut file = File::create(std::path::Path::new(r"prout.png")).unwrap();
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
            //     std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0)).unwrap();
            //     let w = &mut std::io::BufWriter::new(&mut file);
            //     let mut encoder = png::Encoder::new(w, WIDTH.into(), HEIGHT.into());
            //     encoder.set_color(png::ColorType::Grayscale);
            //     encoder.set_depth(png::BitDepth::Two);
            //     let mut writer = encoder.write_header().unwrap();

            //     writer.write_image_data(&current_frame).unwrap(); // Save
            // }

            if emulator.get_ppu().get_ly() == HEIGHT - 1
                && (depth == BitDepth::One
                    && two_bits_depth_to_one_bit_depth_image(&current_frame) == buf.as_slice()
                    || depth == BitDepth::Two && current_frame == buf.as_slice())
            {
                break;
            }
        }
    }
}

#[test]
fn m2_win_en_toggle() {
    mealybug("m2_win_en_toggle");
}

#[test]
fn m3_bgp_change() {
    mealybug("m3_bgp_change");
}

#[test]
fn m3_bgp_change_sprites() {
    mealybug("m3_bgp_change_sprites");
}

#[test]
fn m3_lcdc_bg_en_change() {
    // Different output on gameboy pocket
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_lcdc_bg_en_change.gb",
        "tests/mealybug_pocket_expected/m3_lcdc_bg_en_change.png",
    );
}

#[test]
fn m3_lcdc_bg_en_change2() {
    // Different output on gameboy pocket
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_lcdc_bg_en_change2.gb",
        "tests/mealybug_pocket_expected/m3_lcdc_bg_en_change2.png",
    );
}

#[test]
fn m3_lcdc_bg_map_change() {
    mealybug("m3_lcdc_bg_map_change");
}

#[test]
fn m3_lcdc_bg_map_change2() {
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_lcdc_bg_map_change2.gb",
        "tests/mealybug_pocket_expected/m3_lcdc_bg_map_change2.png",
    );
}

#[test]
fn m3_lcdc_obj_en_change() {
    mealybug("m3_lcdc_bg_map_change");
}

#[test]
fn m3_lcdc_obj_en_change_variant() {
    // Different output on gameboy pocket
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_lcdc_obj_en_change_variant.gb",
        "tests/mealybug_pocket_expected/m3_lcdc_obj_en_change_variant.png",
    );
}

#[test]
fn m3_lcdc_obj_size_change() {
    mealybug("m3_lcdc_obj_size_change");
}

#[test]
fn m3_lcdc_obj_size_change_scx() {
    mealybug("m3_lcdc_obj_size_change_scx");
}

#[test]
fn m3_lcdc_tile_sel_change() {
    mealybug("m3_lcdc_tile_sel_change");
}

#[test]
fn m3_lcdc_tile_sel_change2() {
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_lcdc_tile_sel_change2.gb",
        "tests/mealybug_pocket_expected/m3_lcdc_tile_sel_change2.png",
    );
}

#[test]
fn m3_lcdc_tile_sel_win_change() {
    mealybug("m3_lcdc_tile_sel_win_change");
}

#[test]
fn m3_lcdc_tile_sel_win_change2() {
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_lcdc_tile_sel_win_change2.gb",
        "tests/mealybug_pocket_expected/m3_lcdc_tile_sel_win_change2.png",
    );
}

#[test]
fn m3_lcdc_win_en_change_multiple() {
    mealybug("m3_lcdc_win_en_change_multiple");
}

#[test]
fn m3_lcdc_win_en_change_multiple_wx() {
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_lcdc_win_en_change_multiple_wx.gb",
        "tests/mealybug_pocket_expected/m3_lcdc_win_en_change_multiple_wx.png",
    );
}

#[test]
fn m3_lcdc_win_map_change() {
    mealybug("m3_lcdc_win_map_change");
}

#[test]
fn m3_lcdc_win_map_change2() {
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_lcdc_win_map_change2.gb",
        "tests/mealybug_pocket_expected/m3_lcdc_win_map_change2.png",
    );
}

#[test]
fn m3_obp0_change() {
    mealybug("m3_obp0_change");
}

#[test]
fn m3_scx_high_5_bits() {
    mealybug("m3_scx_high_5_bits");
}

#[test]
#[ignore]
fn m3_scx_low_3_bits() {
    mealybug("m3_scx_low_3_bits");
}

#[test]
#[ignore]
fn m3_scy_change() {
    mealybug("m3_scy_change");
}

#[test]
#[ignore]
fn m3_window_timing() {
    mealybug("m3_window_timing");
}

#[test]
#[ignore]
fn m3_window_timing_wx_0() {
    mealybug("m3_window_timing_wx_0");
}

#[test]
#[ignore]
fn m3_wx_4_change() {
    mealybug("m3_wx_4_change");
}

#[test]
#[ignore]
fn m3_wx_4_change_sprites() {
    mealybug("m3_wx_4_change_sprites");
}

#[test]
#[ignore]
fn m3_wx_5_change() {
    mealybug("m3_wx_5_change");
}

#[test]
#[ignore]
fn m3_wx_6_change() {
    mealybug("m3_wx_6_change");
}
