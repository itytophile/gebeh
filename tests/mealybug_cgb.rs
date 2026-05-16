use std::{fs::File, io::BufReader};

use gebeh::InstantRtc;
use gebeh_core::{Cgb, Emulator, EmulatorExt, HEIGHT, WIDTH};
use gebeh_front_helper::get_mbc;

fn mealybug(name: &str) {
    mealybug_inner(
        &format!("downloads/mealybug-tearoom-tests-master/roms/{name}.gb"),
        &format!("downloads/mealybug-tearoom-tests-master/expected/CPU CGB C/{name}.png"),
    )
}

fn mealybug_inner(rom: &str, expected: &str) {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(expected).unwrap()));

    // convert to RGB 888
    decoder.set_transformations(png::Transformations::EXPAND);

    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut buf).unwrap();
    let buf: Vec<u16> = buf
        .as_chunks::<3>()
        .0
        .iter()
        .map(|[r, g, b]| {
            let r = u16::from(r >> 3);
            let g = u16::from(g >> 3);
            let b = u16::from(b >> 3);
            r | (g << 5) | (b << 10)
        })
        .collect();
    let rom = std::fs::read(rom).unwrap();
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
fn m3_scx_high_5_bits_change2() {
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_scx_high_5_bits_change2.gb",
        "tests/mealybug_pocket_expected/m3_scx_high_5_bits_change2.png",
    );
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
fn m3_scy_change2() {
    mealybug_inner(
        "downloads/mealybug-tearoom-tests-master/roms/m3_scy_change2.gb",
        "tests/mealybug_pocket_expected/m3_scy_change2.png",
    );
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
