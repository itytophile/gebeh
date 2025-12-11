use std::num::NonZeroU8;

use gb_core::{
    HEIGHT, StateMachine, WIDTH,
    cartridge::CartridgeType,
    cpu::Cpu,
    dma::Dma,
    get_factor_8_kib_ram, get_factor_32_kib_rom,
    ppu::{Ppu, Speeder},
    state::{State, WriteOnlyState},
    timer::Timer,
};
use pixels::{PixelsBuilder, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

fn main() {
    color_eyre::install().unwrap();
    env_logger::init();

    // let rom =
    //     std::fs::read("/home/ityt/Documents/git/gb-test-roms/interrupt_time/interrupt_time.gb")
    //         .unwrap();
    // let rom = std::fs::read("/home/ityt/Téléchargements/dmg-acid2.gb").unwrap();
    let rom = std::fs::read("/home/ityt/Téléchargements/pocket/pocket.gb").unwrap();
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
        .compose(Timer::default())
        .compose(Dma::default())
        .compose(Speeder(Ppu::default(), NonZeroU8::new(4).unwrap()));

    let event_loop = EventLoop::new().unwrap();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 4.0, HEIGHT as f64 * 4.0);
        WindowBuilder::new()
            .with_title("Hello Pixels")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let debug_window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 4.0, HEIGHT as f64 * 4.0);
        WindowBuilder::new()
            .with_title("Debug window")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        PixelsBuilder::new(WIDTH.into(), HEIGHT.into(), surface_texture)
            .enable_vsync(false)
            .build()
            .unwrap()
    };

    let mut previous_ly = None;

    event_loop
        .run(|event, elwt| match event {
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                window_id,
                ..
            } if window_id == window.id() => {
                draw_frame_to_window(
                    &mut state,
                    &mut machine,
                    pixels.frame_mut().as_chunks_mut::<4>().0,
                    &mut previous_ly,
                );
                pixels.render().unwrap();
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                window_id,
                ..
            } if window_id == window.id() => {
                pixels.resize_surface(size.width, size.height).unwrap();
            }
            Event::WindowEvent {
                event:
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                elwt.exit();
            }
            _ => {}
        })
        .unwrap();
}

fn draw_frame_to_window(
    state: &mut State,
    mut machine: &mut (impl StateMachine, Speeder<Ppu>),
    pixels: &mut [[u8; 4]],
    previous_ly: &mut Option<u8>,
) {
    loop {
        machine.execute(state).unwrap()(WriteOnlyState::new(state));

        if *previous_ly == Some(state.ly) {
            continue;
        }

        let (_, Speeder(ppu, _)) = &mut machine;

        let Some(scanline) = ppu.get_scanline_if_ready() else {
            continue;
        };

        *previous_ly = Some(state.ly);
        let base = usize::from(state.ly) * usize::from(WIDTH);
        for (pixel, color) in pixels[base..].iter_mut().zip(scanline) {
            *pixel = (*color).into();
        }
        if state.ly == HEIGHT - 1 {
            break;
        }
    }
}

fn draw_to_debug(state: &State, window: &winit::window::Window, pixels: &mut pixels::Pixels<'_>) {}
