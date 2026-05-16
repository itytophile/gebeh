mod emulator_loop;

use std::sync::{Arc, RwLock};

use cpal::traits::HostTrait;
use gebeh::Frame;
use gebeh_core::{
    Cgb, Dmg, HEIGHT, Model, WIDTH,
    joypad::JoypadInput,
    mbc::{CartridgeType, get_factor_8_kib_ram, get_factor_32_kib_rom},
    ppu::scanline::{Scanline, ScanlineBuilder},
};
use gebeh_front_helper::{Compatibility, get_compatibility, get_title_from_rom};
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::emulator_loop::spawn_emulator;

fn get_pixels_from_window(window: &Window, width: u32, height: u32) -> Pixels<'_> {
    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
    PixelsBuilder::new(width, height, surface_texture)
        .enable_vsync(true)
        .build()
        .unwrap()
}

pub enum Mode {
    CgbWhenExplicit,
    DmgWhenPossible,
    AlwaysCgb,
}

fn main() {
    color_eyre::install().unwrap();
    env_logger::init();

    let mut args = std::env::args();

    let rom = std::fs::read(
        args.nth(1)
            .expect("Please provide a path as first argument"),
    )
    .unwrap();

    let mode = match args.next().map(|mode| mode.to_lowercase()).as_deref() {
        Some("cgb") => Mode::AlwaysCgb,
        Some("dmg") => Mode::DmgWhenPossible,
        _ => Mode::CgbWhenExplicit,
    };

    match (get_compatibility(&rom), mode) {
        (Compatibility::Dmg, Mode::CgbWhenExplicit | Mode::DmgWhenPossible)
        | (Compatibility::Both, Mode::DmgWhenPossible) => {
            println!("Running in DMG mode");
            execute::<Dmg>(rom);
        }
        (Compatibility::Cgb, _)
        | (_, Mode::AlwaysCgb)
        | (Compatibility::Both, Mode::CgbWhenExplicit) => {
            println!("Running in CGB mode");
            execute::<Cgb>(rom);
        }
    }
}

fn execute<M: Model>(rom: Vec<u8>) {
    println!("Title: {}", get_title_from_rom(&rom));

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0147--cartridge-type
    let cartridge_type = CartridgeType::try_from(rom[0x147]).unwrap();
    println!("Cartridge type: {cartridge_type:?}");
    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0148--rom-size
    println!("ROM size: {} KiB", get_factor_32_kib_rom(&rom) * 32);
    println!("RAM size: {} KiB", get_factor_8_kib_ram(&rom) * 8);

    let event_loop = EventLoop::new().unwrap();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 4.0, HEIGHT as f64 * 4.0);
        WindowBuilder::new()
            .with_title("gebeh")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = get_pixels_from_window(&window, WIDTH.into(), HEIGHT.into());

    let joypad: Arc<RwLock<JoypadInput>> = Default::default();
    let (tx_frame, rx_frame) = std::sync::mpsc::sync_channel::<
        Frame<<M::ScanlineBuilder as ScanlineBuilder>::Scanline>,
    >(2);

    let shared_joypad = joypad.clone();

    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("failed to find output device");

    let _handle = spawn_emulator::<M>(&device, tx_frame, shared_joypad, rom);

    event_loop
        .run(|event, elwt| match event {
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                window_id,
                ..
            } if window_id == window.id() => {
                for (pixel, color) in pixels.frame_mut().as_chunks_mut::<4>().0.iter_mut().zip(
                    rx_frame
                        .recv()
                        .unwrap()
                        .iter()
                        .flat_map(|scanline| scanline.iter_colors()),
                ) {
                    *pixel = color.into();
                }

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
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Released,
                                physical_key: PhysicalKey::Code(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                let mut joypad = joypad.write().unwrap();
                match keycode {
                    KeyCode::KeyA => joypad.a = false,
                    KeyCode::KeyB => joypad.b = false,
                    KeyCode::ArrowLeft => joypad.left = false,
                    KeyCode::ArrowRight => joypad.right = false,
                    KeyCode::ArrowUp => joypad.up = false,
                    KeyCode::ArrowDown => joypad.down = false,
                    KeyCode::Enter => joypad.start = false,
                    KeyCode::Tab => joypad.select = false,
                    _ => {}
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                let mut joypad = joypad.write().unwrap();
                match keycode {
                    KeyCode::Escape => elwt.exit(),
                    KeyCode::KeyA => joypad.a = true,
                    KeyCode::KeyB => joypad.b = true,
                    KeyCode::ArrowLeft => joypad.left = true,
                    KeyCode::ArrowRight => joypad.right = true,
                    KeyCode::ArrowUp => joypad.up = true,
                    KeyCode::ArrowDown => joypad.down = true,
                    KeyCode::Enter => joypad.start = true,
                    KeyCode::Tab => joypad.select = true,
                    _ => {}
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => elwt.exit(),
            _ => {}
        })
        .unwrap();
}
