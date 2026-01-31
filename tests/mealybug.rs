use std::{fs::File, io::BufReader};

use gebeh::InstantRtc;
use gebeh_core::{Emulator, HEIGHT, WIDTH};
use gebeh_front_helper::get_mbc;

fn mealybug(name: &str) {
    let decoder = png::Decoder::new(BufReader::new(
        File::open(format!(
            "downloads/mealybug-tearoom-tests-master/expected/DMG-blob/{name}.png"
        ))
        .unwrap(),
    ));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut buf).unwrap();
    let rom = std::fs::read(format!(
        "./downloads/mealybug-tearoom-tests-master/roms/{name}.gb"
    ))
    .unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut emulator = Emulator::default();
    let mut previous_ly = None;
    let mut current_frame = [0u8; WIDTH as usize * HEIGHT as usize / 4];
    loop {
        emulator.execute(mbc.as_mut());
        if let Some(scanline) = emulator.get_ppu().get_scanline_if_ready()
            && previous_ly != Some(emulator.state.ly)
        {
            previous_ly = Some(emulator.state.ly);
            current_frame[usize::from(emulator.state.ly) * usize::from(WIDTH) / 4
                ..usize::from(emulator.state.ly + 1) * usize::from(WIDTH) / 4]
                .copy_from_slice(scanline.raw());
            if emulator.state.ly == HEIGHT - 1 && current_frame == buf.as_slice() {
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
    mealybug("m3_lcdc_bg_en_change");
}

#[test]
fn m3_lcdc_bg_map_change() {
    mealybug("m3_lcdc_bg_map_change");
}

#[test]
fn m3_lcdc_obj_en_change() {
    mealybug("m3_lcdc_bg_map_change");
}

#[test]
fn m3_lcdc_obj_en_change_variant() {
    mealybug("m3_lcdc_obj_en_change_variant");
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
fn m3_lcdc_tile_sel_win_change() {
    mealybug("m3_lcdc_tile_sel_win_change");
}

#[test]
fn m3_lcdc_win_en_change_multiple() {
    mealybug("m3_lcdc_win_en_change_multiple");
}

#[test]
fn m3_lcdc_win_en_change_multiple_wx() {
    mealybug("m3_lcdc_win_en_change_multiple_wx");
}

#[test]
fn m3_lcdc_win_map_change() {
    mealybug("m3_lcdc_win_map_change");
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
fn m3_scx_low_3_bits() {
    mealybug("m3_scx_low_3_bits");
}

#[test]
fn m3_scy_change() {
    mealybug("m3_scy_change");
}

#[test]
fn m3_window_timing() {
    mealybug("m3_window_timing");
}

#[test]
fn m3_window_timing_wx_0() {
    mealybug("m3_window_timing_wx_0");
}

#[test]
fn m3_wx_4_change() {
    mealybug("m3_wx_4_change");
}

#[test]
fn m3_wx_4_change_sprites() {
    mealybug("m3_wx_4_change_sprites");
}

#[test]
fn m3_wx_5_change() {
    mealybug("m3_wx_5_change");
}

#[test]
fn m3_wx_6_change() {
    mealybug("m3_wx_6_change");
}
