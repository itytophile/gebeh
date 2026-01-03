use std::{sync::RwLock, time::Duration};

use cpal::traits::HostTrait;
use gebeh::get_mbc;
use gebeh_core::{
    Emulator, HEIGHT,
    joypad::JoypadInput,
    mbc::{CartridgeType, get_factor_8_kib_ram, get_factor_32_kib_rom},
    ppu::Color,
};
use timerfd::{SetTimeFlags, TimerFd, TimerState};

use crate::audio::Audio;

const ITERATION_COUNT: usize = 4194304 / 4 / 256;

// will poll the emulator every 1/256 seconds (because the most frequent sound event is at 256 Hz)
// https://gbdev.io/pandocs/Audio_details.html#div-apu
// Yes the program can reset the div register but I don't think it will be a problem
pub fn run(shared_frame: &RwLock<[[Color; 160]; 144]>, shared_joypad: &RwLock<JoypadInput>) {
    // let rom = std::fs::read("/home/ityt/Téléchargements/dmg-acid2.gb").unwrap();
    // let rom = std::fs::read("/home/ityt/Téléchargements/pocket/pocket.gb").unwrap();
    // let rom = std::fs::read("/home/ityt/Téléchargements/gejmboj/gejmboj.gb").unwrap();
    // let rom = std::fs::read("/home/ityt/Téléchargements/oh_2/oh.gb").unwrap();
    // let rom = std::fs::read("/home/ityt/Téléchargements/20y/20y.gb").unwrap();
    let rom = std::fs::read("/home/ityt/Téléchargements/mallard/mallard.gb").unwrap();
    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0134-0143--title
    let title = &rom[0x134..0x143];
    let end_zero_pos = title
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(title.len());
    let title = str::from_utf8(&title[..end_zero_pos]).unwrap();
    println!("Title: {title}");

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0147--cartridge-type
    let cartridge_type = CartridgeType::try_from(rom[0x147]).unwrap();
    println!("Cartridge type: {cartridge_type:?}");
    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0148--rom-size
    println!("ROM size: {} KiB", get_factor_32_kib_rom(&rom) * 32);
    println!("RAM size: {} KiB", get_factor_8_kib_ram(&rom) * 8);

    // don't forget to slice the vec or you will clone it for each save state
    let mut mbc = get_mbc(rom.as_slice()).unwrap();
    let mut emulator = Emulator::default();

    if let Ok(file) = std::fs::read(format!("{title}.save")) {
        log::info!("Saved data found!");
        mbc.load_saved_ram(&file);
    }

    if let Ok(file) = std::fs::read(format!("{title}.extra.save")) {
        log::info!("Extra data found!");
        mbc.load_additional_data(&file);
    }

    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("failed to find output device");

    let mut audio = Audio::new(&device);

    let mut tfd = TimerFd::new().unwrap();

    tfd.set_state(
        TimerState::Periodic {
            // doesn't work with Duration::ZERO for whatever reason
            current: Duration::from_micros(1),
            interval: Duration::from_micros(1_000_000 / 256),
        },
        SetTimeFlags::Default,
    );

    let mut frame = [[Color::Black; 160]; 144];

    let mut i = 0u8;

    loop {
        tfd.read();
        for _ in 0..ITERATION_COUNT {
            emulator.execute(mbc.as_mut());
            let Some(scanline) = emulator.get_ppu().get_scanline_if_ready() else {
                continue;
            };
            frame[usize::from(emulator.state.ly)] = *scanline;
            if emulator.state.ly == HEIGHT - 1 {
                *shared_frame.write().unwrap() = frame;
            }
        }
        audio.update_sound(emulator.get_apu().clone());

        if i.is_multiple_of(4) {
            // read inputs at 64Hz
            *emulator.get_joypad_mut() = *shared_joypad.read().unwrap();
        }

        i = i.wrapping_add(1);
    }
}
