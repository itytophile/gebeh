#![deny(clippy::all)]

mod mbc;
mod oscillator;
mod wasm_audio;

use error_iter::ErrorIter;
use gebeh_core::apu::Sampler;
use gebeh_core::mbc::Mbc;
use gebeh_core::Emulator;
use log::error;
use pixels::{PixelsBuilder, SurfaceTexture};
use std::cell::Cell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{LazyLock, RwLock};
use wasm_bindgen::prelude::*;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopProxy};
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::mbc::{get_mbc, CloneMbc};
use crate::oscillator::{Oscillator, Params};

// with rust flags specified in .cargo/config.toml, the app is slow with the default allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    fern::Dispatch::new()
        .level(log::LevelFilter::Info)
        .chain(fern::Output::call(console_log::log))
        .apply()
        .unwrap();
}

/// Retrieve current width and height dimensions of browser client window
fn get_window_size() -> LogicalSize<f64> {
    let client_window = web_sys::window().unwrap();
    LogicalSize::new(
        client_window.inner_width().unwrap().as_f64().unwrap(),
        client_window.inner_height().unwrap().as_f64().unwrap(),
    )
}

#[derive(Default)]
struct LinearFeedbackShiftRegister(u16);

impl LinearFeedbackShiftRegister {
    fn tick(&mut self, short_mode: bool) -> u8 {
        // https://gbdev.io/pandocs/Audio_details.html#noise-channel-ch4
        let new_value = (self.0 & 1 != 0) == (self.0 & 0b10 != 0);
        self.0 = self.0 & 0x7fff | ((new_value as u16) << 15);
        if short_mode {
            self.0 = self.0 & 0xff7f | ((new_value as u16) << 7)
        }
        let shifted_out = self.0 & 1;
        self.0 >>= 1;
        shifted_out as u8
    }
}

fn get_noise(is_short: bool) -> Vec<u8> {
    let mut lfsr = LinearFeedbackShiftRegister::default();
    let mut already_seen = HashSet::new();
    let mut noise = Vec::new();
    while already_seen.insert(lfsr.0) {
        noise.push(lfsr.tick(is_short));
    }
    noise
}

#[wasm_bindgen]
pub struct Proxy(EventLoopProxy<Vec<u8>>);

#[wasm_bindgen]
impl Proxy {
    pub fn send_file(&self, file: Vec<u8>) {
        self.0.send_event(file).unwrap()
    }
}

#[wasm_bindgen]
pub fn init_window() -> Proxy {
    let event_loop = EventLoopBuilder::<Vec<u8>>::with_user_event()
        .build()
        .unwrap();
    let proxy = Proxy(event_loop.create_proxy());

    wasm_bindgen_futures::spawn_local(run(event_loop));

    proxy
}

thread_local! {
    static IS_AUDIO_INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

static SAMPLER: LazyLock<RwLock<Sampler>> = LazyLock::new(Default::default);

#[wasm_bindgen]
pub async fn init_audio() {
    if IS_AUDIO_INITIALIZED.get() {
        return;
    }
    log::info!("Audio init!");
    IS_AUDIO_INITIALIZED.set(false);
    let params: &'static Params = Box::leak(Box::default());
    let mut osc = Oscillator::new(params);

    if let Err(err) = wasm_audio::wasm_audio(Box::new(move |left| {
        osc.process(left)
    }))
    .await
    {
        log::error!("Can't init audio");
        web_sys::console::error_1(&err);
    }
}

async fn run(event_loop: EventLoop<Vec<u8>>) {
    let noise = get_noise(false).leak();
    let short_noise = get_noise(true).leak();
    let window = {
        let size = LogicalSize::new(gebeh_core::WIDTH as f64, gebeh_core::HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Hello Pixels + Web")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .expect("WindowBuilder error")
    };

    let window = Rc::new(window);

    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowExtWebSys;

    // Attach winit canvas to body element
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .and_then(|body| {
            body.append_child(&web_sys::Element::from(window.canvas().unwrap()))
                .ok()
        })
        .expect("couldn't append canvas to document body");

    // Listen for resize event on browser client. Adjust winit window dimensions
    // on event trigger
    let closure = wasm_bindgen::closure::Closure::wrap(Box::new({
        let window = Rc::clone(&window);
        move |_e: web_sys::Event| {
            let _ = window.request_inner_size(get_window_size());
        }
    }) as Box<dyn FnMut(_)>);
    web_sys::window()
        .unwrap()
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    // Trigger initial resize event
    let _ = window.request_inner_size(get_window_size());

    let mut input = WinitInputHelper::new();
    let mut pixels = {
        let window_size = get_window_size().to_physical::<u32>(window.scale_factor());

        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.as_ref());
        let builder = PixelsBuilder::new(
            gebeh_core::WIDTH.into(),
            gebeh_core::HEIGHT.into(),
            surface_texture,
        );

        let builder = {
            // Web targets do not support the default texture format
            let texture_format = pixels::wgpu::TextureFormat::Rgba8Unorm;
            builder
                .texture_format(texture_format)
                .surface_texture_format(texture_format)
        };

        builder.build_async().await.expect("Pixels error")
    };

    let mut mbc_and_emulator: Option<(Box<dyn CloneMbc>, Emulator)> = None;

    let res = event_loop.run(|event, elwt| {
        // Handle input events
        if input.update(&event) && (input.key_pressed(KeyCode::Escape) || input.close_requested()) {
            elwt.exit();
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                if let Some((mbc, emulator)) = &mut mbc_and_emulator {
                    draw_emulator(
                        mbc.as_mut(),
                        emulator,
                        pixels.frame_mut().as_chunks_mut::<4>().0,
                    );

                    if let Err(err) = pixels.render() {
                        log_error("pixels.render", err);
                        elwt.exit();
                        return;
                    }
                };

                window.request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Resize the window
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    elwt.exit();
                }
            }
            Event::UserEvent(file) => {
                log::info!("New file ! size = {}", file.len());
                mbc_and_emulator = Some((get_mbc(file).unwrap(), Emulator::default()));
            }

            _ => (),
        }
    });
    res.unwrap();
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}

fn draw_emulator(mbc: &mut dyn Mbc, emulator: &mut Emulator, pixels: &mut [[u8; 4]]) {
    let start = web_time::Instant::now();
    while start.elapsed() <= web_time::Duration::from_millis(33) {
        emulator.execute(mbc);

        let Some(scanline) = emulator.get_ppu().get_scanline_if_ready() else {
            continue;
        };

        let base = usize::from(emulator.state.ly) * usize::from(gebeh_core::WIDTH);
        for (pixel, color) in pixels[base..].iter_mut().zip(scanline) {
            *pixel = (*color).into();
        }
        if emulator.state.ly == gebeh_core::HEIGHT - 1 {
            break;
        }
    }
}
