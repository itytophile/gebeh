use std::num::NonZeroU8;

use gb_core::{
    HEIGHT, StateMachine, WIDTH,
    cartridge::CartridgeType,
    cpu::Cpu,
    dma::Dma,
    get_factor_8_kib_ram, get_factor_32_kib_rom,
    ppu::{LcdControl, Ppu, Speeder, get_bg_win_tile, get_color_from_line, get_line_from_tile},
    state::{State, WriteOnlyState},
    timer::Timer,
};
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

const DEBUG_TILE_COL_COUNT: u8 = 16;
// must be divisible by three because there are three blocks of tiles
// https://gbdev.io/pandocs/Tile_Data.html
const DEBUG_TILE_ROW_COUNT: u8 = 24;
const DEBUG_TILE_WIDTH: u8 = DEBUG_TILE_COL_COUNT * 8;
const DEBUG_TILE_HEIGHT: u8 = DEBUG_TILE_ROW_COUNT * 8;

const DEBUG_TILE_MAP_COL_COUNT: u8 = 32;
const DEBUG_TILE_MAP_ROW_COUNT: u8 = 64;
const DEBUG_TILE_MAP_WIDTH: u16 = DEBUG_TILE_MAP_COL_COUNT as u16 * 8;
const DEBUG_TILE_MAP_HEIGHT: u16 = DEBUG_TILE_MAP_ROW_COUNT as u16 * 8;

fn get_pixels_from_window(window: &Window, width: u32, height: u32) -> Pixels<'_> {
    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
    PixelsBuilder::new(width.into(), height.into(), surface_texture)
        .build()
        .unwrap()
}

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
            .with_title("Emulator")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let debug_window = {
        let size = LogicalSize::new(DEBUG_TILE_WIDTH as f64, DEBUG_TILE_HEIGHT as f64);
        let scaled_size = LogicalSize::new(
            DEBUG_TILE_WIDTH as f64 * 4.0,
            DEBUG_TILE_HEIGHT as f64 * 4.0,
        );
        WindowBuilder::new()
            .with_title("Tile debug")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let debug_tile_map_window = {
        let size = LogicalSize::new(DEBUG_TILE_MAP_WIDTH as f64, DEBUG_TILE_MAP_HEIGHT as f64);
        let scaled_size = LogicalSize::new(
            DEBUG_TILE_MAP_WIDTH as f64 * 4.0,
            DEBUG_TILE_MAP_HEIGHT as f64 * 4.0,
        );
        WindowBuilder::new()
            .with_title("Tile debug")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let debug_tile_map_pixels = get_pixels_from_window(
        &debug_tile_map_window,
        DEBUG_TILE_MAP_WIDTH.into(),
        DEBUG_TILE_MAP_HEIGHT.into(),
    );

    let mut debug_pixels = get_pixels_from_window(
        &debug_window,
        DEBUG_TILE_WIDTH.into(),
        DEBUG_TILE_HEIGHT.into(),
    );

    let mut pixels = get_pixels_from_window(&window, WIDTH.into(), HEIGHT.into());

    let mut previous_ly = None;

    event_loop
        .run(|event, elwt| match event {
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                window_id,
                ..
            } if window_id == window.id() => {
                draw_emulator(
                    &mut state,
                    &mut machine,
                    pixels.frame_mut().as_chunks_mut::<4>().0,
                    &mut previous_ly,
                );
                pixels.render().unwrap();
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                window_id,
                ..
            } if window_id == debug_window.id() => {
                draw_tiles_debug(&state, debug_pixels.frame_mut().as_chunks_mut::<4>().0);
                debug_pixels.render().unwrap();
                debug_window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                window_id,
                ..
            } if window_id == debug_tile_map_window.id() => {
                draw_tile_map_debug(&state, debug_pixels.frame_mut().as_chunks_mut::<4>().0);
                debug_tile_map_pixels.render().unwrap();
                debug_tile_map_window.request_redraw();
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

fn draw_emulator(
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

fn draw_tiles_debug(state: &State, pixels: &mut [[u8; 4]]) {
    let (tiles, _) = state.video_ram[..0x1800].as_chunks::<16>();
    for (index, tile) in tiles.iter().enumerate() {
        // 0xe1 because pocket uses that. We shouldn't use the bgp register because it's not stable
        draw_tile(pixels, index, tile, DEBUG_TILE_COL_COUNT, 0xe1);
    }
}

fn draw_tile(
    pixels: &mut [[u8; 4]],
    index: usize,
    tile: &[u8; 16],
    tile_col_count: u8,
    palette: u8,
) {
    for (y, line) in (0..8).map(|y| (y, get_line_from_tile(tile, y))) {
        for (x, color) in (0..8).map(|x| (x, get_color_from_line(line, x))) {
            let tile_x = index % usize::from(tile_col_count);
            let tile_y = index / usize::from(tile_col_count);
            let pixel_x = tile_x * 8 + usize::from(x);
            let pixel_y = tile_y * 8 + usize::from(y);
            pixels[pixel_y * usize::from(tile_col_count) * 8 + pixel_x] =
                color.get_color(palette).into();
        }
    }
}

fn draw_tile_map_debug(state: &State, pixels: &mut [[u8; 4]]) {
    for (index, tile_index) in state.video_ram[0x1800..].iter().copied().enumerate() {
        let tile = get_bg_win_tile(
            state.video_ram[..0x1800].try_into().unwrap(),
            tile_index,
            !state.lcd_control.contains(LcdControl::BG_AND_WINDOW_TILES),
        );
        draw_tile(
            pixels,
            index,
            tile,
            DEBUG_TILE_MAP_COL_COUNT,
            state.bgp_register,
        );
    }
}
