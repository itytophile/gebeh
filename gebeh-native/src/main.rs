mod emulator_loop;

use std::sync::{Arc, RwLock};

use cpal::traits::HostTrait;
use gebeh::Frame;
use gebeh_core::{HEIGHT, WIDTH, joypad::JoypadInput};
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

fn main() {
    color_eyre::install().unwrap();
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 4.0, HEIGHT as f64 * 4.0);
        WindowBuilder::new()
            .with_title("Emulator")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = get_pixels_from_window(&window, WIDTH.into(), HEIGHT.into());

    let joypad: Arc<RwLock<JoypadInput>> = Default::default();
    let (tx_frame, rx_frame) = std::sync::mpsc::sync_channel::<Frame>(2);

    let shared_joypad = joypad.clone();

    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("failed to find output device");

    let _handle = spawn_emulator(&device, tx_frame, shared_joypad);

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
