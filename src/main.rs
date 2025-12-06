use std::{
    num::NonZeroU8,
    time::{Duration, Instant},
};

use minifb::{Key, Scale, Window, WindowOptions};
use testouille_emulator_future::{
    StateMachine,
    cartridge::CartridgeType,
    cpu::Cpu,
    get_factor_8_kib_ram, get_factor_32_kib_rom,
    ppu::{Ppu, Speeder},
    state::{State, WriteOnlyState},
    timer::Timer,
};

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

fn main() {
    color_eyre::install().unwrap();
    // let rom =
    //     std::fs::read("/home/ityt/Documents/git/gb-test-roms/interrupt_time/interrupt_time.gb")
    //         .unwrap();
    let rom = std::fs::read("/home/ityt/Téléchargements/dmg-acid2.gb").unwrap();
    // let rom =
    //     std::fs::read("/home/ityt/Documents/git/gb-test-roms/cpu_instrs/individual/10-bit ops.gb")
    //         .unwrap();
    // let rom =
    //     std::fs::read("/home/ityt/Documents/git/gb-test-roms/cpu_instrs/cpu_instrs.gb").unwrap();
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

    let mut state = State::new(rom.leak());
    // the machine should not be affected by the composition order
    let mut machine = Cpu::default()
        .compose(Speeder(Ppu::default(), NonZeroU8::new(4).unwrap()))
        .compose(Timer::default());

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: false,
            scale: Scale::X4,
            ..Default::default()
        },
    )
    .unwrap();

    let mut is_changed = false;

    let check_duration = Duration::from_millis(100);
    let mut last_checked = Instant::now();
    let mut previous_ly = None;

    loop {
        machine.execute(&state).unwrap()(WriteOnlyState::new(&mut state));
        if last_checked.elapsed() > check_duration {
            window.update();
            last_checked = Instant::now();
            if !window.is_open() || window.is_key_down(Key::Escape) {
                return;
            }
        }
        let ((_, Speeder(ppu, _)), _) = &mut machine;
        if let Some(scanline) = ppu.get_scanline_if_ready()
            && previous_ly != Some(state.ly)
        {
            previous_ly = Some(state.ly);
            let base = usize::from(state.ly) * WIDTH;
            for (a, b) in buffer[base..].iter_mut().zip(scanline) {
                let b = u32::from(*b);
                if *a != b {
                    is_changed = true;
                }
                *a = b;
            }
            if usize::from(state.ly) == HEIGHT - 1 && is_changed {
                window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
                is_changed = false;
            }
        }
    }
}
