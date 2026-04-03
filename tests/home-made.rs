use std::{fs::File, io::BufReader};

use gebeh::InstantRtc;
use gebeh_core::{Emulator, HEIGHT, WIDTH, joypad::JoypadInput};
use gebeh_front_helper::get_mbc;
use gebeh_network::RollbackSerial;

fn home_made(name: &str) {
    let rom = std::fs::read(format!("./gebeh-test-roms/{name}.gb")).unwrap();
    let rom = rom.as_slice();
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut emulator = Emulator::default();

    loop {
        emulator.execute(mbc.as_mut());
        match emulator.get_cpu().current_opcode {
            // ld b, b
            0x40 => break,
            // ld c, c
            0x49 => panic!("ld c, c instead of ld b, b"),
            _ => {}
        }
    }
}

fn home_made_ppu(name: &str) {
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
    // let path = std::path::Path::new(r"prout.png");
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
            //     std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0)).unwrap();
            //     let w = &mut std::io::BufWriter::new(&mut file);
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
    home_made_ppu("stat_mode_2_palette_screen_edges");
}

#[test]
fn lyc_palette_screen_edges() {
    home_made_ppu("lyc_palette_screen_edges");
}

#[test]
fn halt_lyc_palette_screen_edges() {
    home_made_ppu("halt_lyc_palette_screen_edges");
}

#[test]
fn halt_stat_mode_2_palette_screen_edges() {
    home_made_ppu("halt_stat_mode_2_palette_screen_edges");
}

#[test]
fn serial_master_transfer_timing() {
    home_made("serial_master_transfer_timing");
}

#[test]
fn serial_master_overclock() {
    home_made("serial_master_overclock");
}

#[test]
fn serial_exchange() {
    env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .init();
    let rom = &*std::fs::read("./gebeh-test-roms/serial.gb").unwrap().leak();
    let (_, mut slave_mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut slave_emulator = Emulator::default();
    let (_, mut master_mbc) = get_mbc::<_, InstantRtc>(rom).unwrap();
    let mut master_emulator = Emulator::default();

    let mut slave_rollback = RollbackSerial::default();
    let mut master_rollback = RollbackSerial::default();

    let mut messages_from_slave = Vec::new();
    let mut messages_from_master = Vec::new();

    // wait for ld a, a
    loop {
        messages_from_slave.extend(
            slave_rollback.execute_and_take_snapshot(&mut slave_emulator, slave_mbc.as_mut()),
        );
        if let 0x7f = slave_emulator.get_cpu().current_opcode {
            break;
        }
    }
    loop {
        messages_from_master.extend(
            master_rollback.execute_and_take_snapshot(&mut master_emulator, master_mbc.as_mut()),
        );
        if let 0x7f = master_emulator.get_cpu().current_opcode {
            break;
        }
    }
    master_emulator.set_joypad(JoypadInput {
        start: true,
        ..Default::default()
    });

    for msg in messages_from_slave.drain(..) {
        master_rollback.add_message(&msg);
    }
    for msg in messages_from_master.drain(..) {
        slave_rollback.add_message(&msg);
    }

    log::info!("prout");

    while slave_emulator.get_cpu().h != 7 || master_emulator.get_cpu().h != 7 {
        log::info!("slave");
        slave_rollback.rollback_if_necessary(&mut slave_emulator, &mut slave_mbc);
        log::info!("after deviation");
        messages_from_slave.extend(
            slave_rollback.execute_and_take_snapshot(&mut slave_emulator, slave_mbc.as_mut()),
        );
        log::info!("master");
        master_rollback.rollback_if_necessary(&mut master_emulator, &mut master_mbc);
        messages_from_master.extend(
            master_rollback.execute_and_take_snapshot(&mut master_emulator, master_mbc.as_mut()),
        );
        for msg in messages_from_slave.drain(..) {
            master_rollback.add_message(&msg);
        }
        for msg in messages_from_master.drain(..) {
            slave_rollback.add_message(&msg);
        }
    }

    assert_eq!(slave_emulator.get_cpu().b, 7);
    assert_eq!(master_emulator.get_cpu().b, 7)
}
