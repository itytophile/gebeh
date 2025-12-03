use std::num::{NonZeroU8, NonZeroU16};

use bitflags::Flags;

use crate::{
    StateMachine,
    gpu::{self, LcdControl},
    ic::Ints,
    state::{LcdStatus, State, WriteOnlyState},
};

pub enum Ppu2 {
    OamScan { remaining_dots: NonZeroU8 },                // <= 80
    DrawingPixels { dots_count: u16 },                    // <= 289
    HorizontalBlank { remaining_dots: NonZeroU8 },        // <= 204
    VerticalBlankScanline { remaining_dots: NonZeroU16 }, // <= 456
}

const OAM_SCAN_DURATION: NonZeroU8 = NonZeroU8::new(80).unwrap();
const VERTICAL_BLANK_SCANLINE_DURATION: NonZeroU16 = NonZeroU16::new(456).unwrap();

impl Default for Ppu2 {
    fn default() -> Self {
        Self::OamScan {
            remaining_dots: OAM_SCAN_DURATION,
        }
    }
}

// Tile data

const TILE_LENGTH: u8 = 16;

type TileVram = [u8; 0x1800];
type Tile = [u8; 16];
type Line = [u8; 2];

pub enum ColorIndex {
    Zero = 0b00,
    One = 0b01,
    Two = 0b10,
    Three = 0b11,
}

impl ColorIndex {
    pub fn new(least_significant_bit: bool, most_significant_bit: bool) -> Self {
        match (most_significant_bit, least_significant_bit) {
            (true, true) => Self::Three,
            (true, false) => Self::Two,
            (false, true) => Self::One,
            (false, false) => Self::Zero,
        }
    }
}

// https://gbdev.io/pandocs/Tile_Data.html#vram-tile-data
fn get_object_tile(vram: &TileVram, index: u8) -> &Tile {
    let base = usize::from(index) * usize::from(TILE_LENGTH);
    vram[base..base + usize::from(TILE_LENGTH)]
        .try_into()
        .unwrap()
}

fn get_bg_win_tile(vram: &TileVram, index: u8, is_signed_addressing: bool) -> &Tile {
    let base = if is_signed_addressing {
        0x1000usize.strict_add_signed(isize::from(index.cast_signed()) * isize::from(TILE_LENGTH))
    } else {
        usize::from(index) * usize::from(TILE_LENGTH)
    };
    vram[base..base + usize::from(TILE_LENGTH)]
        .try_into()
        .unwrap()
}

// Tile maps

type TileMap = [u8; 0x400]; // 32 * 32 Tile indexes

// OAM

type OAM = [u8; 0xa0];

bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
    pub struct ObjectFlags: u8 {
        const PRIORITY = 1 << 7;
        const Y_FLIP = 1 << 6;
        const X_FLIP = 1 << 5;
        const DMG_PALETTE = 1 << 4;
    }
}

struct ObjectAttribute {
    y: u8,
    x: u8,
    tile_index: u8,
    flags: ObjectFlags,
}

impl From<[u8; 4]> for ObjectAttribute {
    fn from([y, x, tile_index, flags]: [u8; 4]) -> Self {
        Self {
            y,
            x,
            tile_index,
            flags: ObjectFlags::from_bits_retain(flags),
        }
    }
}

// Pixel FIFO

#[derive(Clone, Copy)]
struct Window {
    x: u8,
    y: u8,
}
#[derive(Clone, Copy)]
struct Scanline {
    // 0 <= x < 160
    x: u8,
    // 0 <= y < 144
    y: u8,
}

#[derive(Clone, Copy)]
struct Background {
    // 0 < x < 256
    x: u8,
    // 0 < y < 256
    y: u8,
}

impl Window {
    fn is_visible(self) -> bool {
        self.x <= 166 && self.y <= 143
    }
}

// A pixel inside the 256x256 pixels picture held by the tile map
#[derive(Clone, Copy)]
struct PicturePixel {
    x: u8,
    y: u8,
}

impl PicturePixel {
    fn get_relative_tile_map_index(self) -> u16 {
        u16::from(self.x / 8) + u16::from(self.y) * 4 // x / 8 + y / 8 * 32
    }
}

// TODO If WX is set to 166, the window will span the entirety of the following scanline.

// Window is just a static image that can be moved on the screen
// so it's always refering to the same tiles

// https://gbdev.io/pandocs/Scrolling.html#ff4aff4b--wy-wx-window-y-position-x-position-plus-7
// The Window is visible (if enabled) when both coordinates are in the ranges WX=0..166, WY=0..143 respectively.
// Values WX=7, WY=0 place the Window at the top left of the screen, completely covering the background.

// Background is not a static image, it can scroll over the tiles.

// https://gbdev.io/pandocs/Tile_Maps.html#tile-indexes
// Since one tile has 8×8 pixels, each map holds a 256×256 pixels picture.
// Only 160×144 of those pixels are displayed on the LCD at any given time.

// https://gbdev.io/pandocs/pixel_fifo.html#get-tile
fn get_picture_pixel_and_tile_map_address(
    lcdc: LcdControl,
    scanline: Scanline,
    window: Window,
    background: Background,
) -> (PicturePixel, u16) {
    assert!(scanline.x < 160);
    assert!(scanline.y < 144);

    if window.is_visible()
        && let (Some(x), Some(y)) = (
            (scanline.x + 7).checked_sub(window.x),
            scanline.y.checked_sub(window.y),
        )
    {
        // is in window
        (
            PicturePixel { x, y },
            if lcdc.contains(LcdControl::WINDOW_TILE_MAP) {
                0x9c00
            } else {
                0x9800
            },
        )
    } else {
        (
            PicturePixel {
                x: scanline.x.wrapping_sub(background.x),
                y: scanline.y.wrapping_sub(background.y),
            },
            if lcdc.contains(LcdControl::BG_TILE_MAP) {
                0x9c00
            } else {
                0x9800
            },
        )
    }
}

// TODO if the PPU’s access to VRAM is blocked then the tile data is read as $FF
fn get_tile_data_low(tile_id: u8, vram: &TileVram) {
    get_bg_win_tile()
}

// one iteration = one dot = (1/4 M-cyle DMG)
impl StateMachine for Ppu2 {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        let mut ly = state.ly;
        let mut mode_changed = false;
        let lcd_status = state.lcd_status;

        match self {
            Ppu2::OamScan { remaining_dots } => {
                if let Some(dots) = NonZeroU8::new(remaining_dots.get() - 1) {
                    *remaining_dots = dots;
                } else {
                    mode_changed = true;
                    *self = Ppu2::DrawingPixels { dots_count: 0 };
                }
            }
            Ppu2::DrawingPixels { dots_count } => todo!(),
            Ppu2::HorizontalBlank { remaining_dots } => {
                if let Some(dots) = NonZeroU8::new(remaining_dots.get() - 1) {
                    *remaining_dots = dots;
                } else {
                    ly += 1;
                    mode_changed = true;
                    if ly == 144 {
                        *self = Ppu2::VerticalBlankScanline {
                            remaining_dots: VERTICAL_BLANK_SCANLINE_DURATION,
                        };
                    } else {
                        *self = Ppu2::OamScan {
                            remaining_dots: OAM_SCAN_DURATION,
                        }
                    }
                }
            }
            Ppu2::VerticalBlankScanline { remaining_dots } => {
                if let Some(dots) = NonZeroU16::new(remaining_dots.get() - 1) {
                    *remaining_dots = dots;
                } else {
                    if ly == 153 {
                        mode_changed = true;
                        ly = 0;
                        *self = Ppu2::OamScan {
                            remaining_dots: OAM_SCAN_DURATION,
                        }
                    } else {
                        ly += 1;
                        *self = Ppu2::VerticalBlankScanline {
                            remaining_dots: VERTICAL_BLANK_SCANLINE_DURATION,
                        }
                    }
                }
            }
        };
        Some(move |mut state: WriteOnlyState| {
            state.set_ly(ly);

            if !mode_changed {
                return;
            }

            let (mode, request_interrupt) = match self {
                Ppu2::OamScan { .. } => {
                    (gpu::Mode::OamScan, lcd_status.contains(LcdStatus::OAM_INT))
                }
                Ppu2::DrawingPixels { .. } => (gpu::Mode::Drawing, false),
                Ppu2::HorizontalBlank { .. } => (
                    gpu::Mode::HBlank,
                    lcd_status.contains(LcdStatus::HBLANK_INT),
                ),
                Ppu2::VerticalBlankScanline { .. } => (
                    gpu::Mode::VBlank,
                    lcd_status.contains(LcdStatus::VBLANK_INT),
                ),
            };

            state.set_ppu_mode(mode);

            if request_interrupt {
                state.get_if_mut().insert(Ints::LCD);
            }
        })
    }
}
