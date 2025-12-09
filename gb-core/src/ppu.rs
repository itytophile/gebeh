use core::num::{NonZeroU8, NonZeroU16};

use arrayvec::ArrayVec;

use crate::{
    StateMachine,
    ic::Ints,
    state::{LcdStatus, State, VIDEO_RAM, WriteOnlyState},
};

pub enum Ppu {
    OamScan {
        remaining_dots: NonZeroU8,
        // https://gbdev.io/pandocs/Scrolling.html#window
        wy_condition: bool,
        internal_y_window_counter: u8,
    }, // <= 80
    DrawingPixels {
        dots_count: u16,
        scanline: [Color; 160],
        wy_condition: bool,
        internal_y_window_counter: u8,
    }, // <= 289
    HorizontalBlank {
        remaining_dots: NonZeroU8,
        wy_condition: bool,
        internal_y_window_counter: u8,
    }, // <= 204
    VerticalBlankScanline {
        remaining_dots: NonZeroU16,
        // no wy_condition because vblank means the frame ends
    }, // <= 456
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq, Default)]
    pub struct LcdControl: u8 {
        const LCD_PPU_ENABLE = 1 << 7;
        const WINDOW_TILE_MAP = 1 << 6;
        const WINDOW_ENABLE = 1 << 5;
        const BG_AND_WINDOW_TILES = 1 << 4;
        const BG_TILE_MAP = 1 << 3;
        const OBJ_SIZE = 1 << 2;
        const OBJ_ENABLE = 1 << 1;
        const BG_AND_WINDOW_ENABLE = 1;
    }
}

const OAM_SCAN_DURATION: NonZeroU8 = NonZeroU8::new(80).unwrap();
const VERTICAL_BLANK_SCANLINE_DURATION: NonZeroU16 = NonZeroU16::new(456).unwrap();

impl Default for Ppu {
    fn default() -> Self {
        Self::OamScan {
            remaining_dots: OAM_SCAN_DURATION,
            wy_condition: false,
            internal_y_window_counter: 0,
        }
    }
}

// Tile data

const TILE_LENGTH: u8 = 16;

type TileVram = [u8; 0x1800];
type TileVramObj = [u8; 0x1000];
type Tile = [u8; 16];

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ColorIndex {
    Zero,
    One,
    Two,
    Three,
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

    pub fn get_color(self, palette: u8) -> Color {
        let shift: u8 = match self {
            ColorIndex::Zero => 0,
            ColorIndex::One => 2,
            ColorIndex::Two => 4,
            ColorIndex::Three => 6,
        };
        match (palette >> shift) & 0b11 {
            0 => Color::White,
            1 => Color::LightGray,
            2 => Color::DarkGray,
            _ => Color::Black,
        }
    }
}

fn get_line_from_tile(tile: &Tile, y: u8) -> [u8; 2] {
    assert!(y < 8);
    tile[usize::from(y * 2)..usize::from((y + 1) * 2)]
        .try_into()
        .unwrap()
}

fn get_color_from_line(line: [u8; 2], x: u8) -> ColorIndex {
    assert!(x < 8);
    ColorIndex::new((line[0] & (0x80 >> x)) != 0, (line[1] & (0x80 >> x)) != 0)
}

#[must_use]
fn get_color_from_tile(tile: &Tile, x: u8, y: u8) -> ColorIndex {
    get_color_from_line(get_line_from_tile(tile, y), x)
}

// https://gbdev.io/pandocs/Tile_Data.html#vram-tile-data
#[must_use]
fn get_object_tile(vram: &TileVramObj, index: u8) -> &Tile {
    let base = usize::from(index) * usize::from(TILE_LENGTH);
    vram[base..base + usize::from(TILE_LENGTH)]
        .try_into()
        .unwrap()
}

#[must_use]
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

bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
    pub struct ObjectFlags: u8 {
        const PRIORITY = 1 << 7;
        const Y_FLIP = 1 << 6;
        const X_FLIP = 1 << 5;
        const DMG_PALETTE = 1 << 4;
    }
}

#[derive(Clone, Copy)]
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
    internal_y_window_counter: Option<u8>,
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

// A pixel inside the 256x256 pixels picture held by the tile map
#[derive(Clone, Copy, Debug)]
struct PicturePixel {
    x: u8,
    y: u8,
}

impl PicturePixel {
    fn get_relative_tile_map_index(self) -> u16 {
        u16::from(self.x / 8) + u16::from(self.y) / 8 * 32 // don't simplify this product lol
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
#[must_use]
fn get_picture_pixel_and_tile_map_address(
    lcdc: LcdControl,
    scanline: Scanline,
    window: Window,
    background: Background,
) -> (PicturePixel, u16) {
    assert!(scanline.x < 160);
    assert!(scanline.y < 144);

    if let Some(y) = window.internal_y_window_counter {
        // is in window
        (
            PicturePixel {
                x: scanline.x + 7 - window.x, // no overflow because wx_condition was triggered
                y,
            },
            if lcdc.contains(LcdControl::WINDOW_TILE_MAP) {
                0x9c00
            } else {
                0x9800
            },
        )
    } else {
        (
            PicturePixel {
                x: scanline.x.wrapping_add(background.x),
                y: scanline.y.wrapping_add(background.y),
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

pub trait StateMachine2 {
    type WorkState;
    fn get_work_state(state: &State) -> Self::WorkState;
    fn execute(&mut self, work_state: &mut Self::WorkState, state: &State);
    fn commit(&self, work_state: Self::WorkState, state: WriteOnlyState);
}

pub struct PpuWorkState {
    ly: u8,
    is_requesting_lcd_int: bool,
    is_requesting_vblank_int: bool,
}

impl Ppu {
    #[must_use]
    pub fn get_scanline_if_ready(&self) -> Option<&[Color; 160]> {
        match self {
            Self::DrawingPixels {
                scanline,
                dots_count,
                ..
            } if *dots_count > 0 => Some(scanline),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_starting_line(&self) -> bool {
        matches!(
            self,
            Ppu::OamScan {
                remaining_dots: OAM_SCAN_DURATION,
                ..
            } | Ppu::VerticalBlankScanline {
                remaining_dots: VERTICAL_BLANK_SCANLINE_DURATION,
            }
        )
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Color {
    White,
    LightGray,
    DarkGray,
    Black,
}

impl From<Color> for u32 {
    fn from(c: Color) -> u32 {
        match c {
            Color::White => 0xffffff,
            Color::LightGray => 0xaaaaaa,
            Color::DarkGray => 0x555555,
            Color::Black => 0,
        }
    }
}

// one iteration = one dot = (1/4 M-cyle DMG)
impl StateMachine2 for Ppu {
    type WorkState = PpuWorkState;

    fn get_work_state(state: &State) -> Self::WorkState {
        PpuWorkState {
            is_requesting_lcd_int: false,
            ly: state.ly,
            is_requesting_vblank_int: false,
        }
    }

    fn execute(&mut self, work_state: &mut Self::WorkState, state: &State) {
        if !state.lcd_control.contains(LcdControl::LCD_PPU_ENABLE) {
            return;
        }

        let mut mode_changed = false;

        if state.lcd_status.contains(LcdStatus::LYC_INT)
            && self.is_starting_line()
            && work_state.ly == state.lyc
        {
            work_state.is_requesting_lcd_int = true;
        }

        match self {
            Ppu::OamScan {
                remaining_dots,
                wy_condition,
                internal_y_window_counter,
            } => {
                // Citation:
                // at some point in this frame the value of WY was equal to LY (checked at the start of Mode 2 only)
                *wy_condition |= *remaining_dots == OAM_SCAN_DURATION && work_state.ly == state.wy;
                if let Some(dots) = NonZeroU8::new(remaining_dots.get() - 1) {
                    *remaining_dots = dots;
                } else {
                    mode_changed = true;
                    *self = Ppu::DrawingPixels {
                        dots_count: 0,
                        scanline: [Color::Black; 160],
                        wy_condition: *wy_condition,
                        internal_y_window_counter: *internal_y_window_counter,
                    };
                }
            }
            Ppu::DrawingPixels {
                dots_count,
                scanline,
                wy_condition,
                internal_y_window_counter,
            } => {
                // if first iteration then draw whole line without thinking
                // TODO: draw the line during the good amount of dots
                if *dots_count == 0 {
                    let mut bg_win_colors = [Option::<Color>::None; 160];

                    if state.lcd_control.contains(LcdControl::BG_AND_WINDOW_ENABLE) {
                        // https://gbdev.io/pandocs/Scrolling.html#window
                        let mut wx_condition = false;
                        for (x, color) in bg_win_colors.iter_mut().enumerate() {
                            let x = u8::try_from(x).unwrap();
                            // Citation:
                            // the current X coordinate being rendered + 7 was equal to WX
                            wx_condition |= x + 7 == state.wx;
                            let scanline = Scanline {
                                x,
                                y: work_state.ly,
                            };
                            let color_index = get_color_bg_win(
                                scanline,
                                state,
                                (wx_condition
                                    && *wy_condition
                                    && state.lcd_control.contains(LcdControl::WINDOW_ENABLE))
                                .then_some(*internal_y_window_counter),
                            );
                            *color = if color_index == ColorIndex::Zero {
                                None
                            } else {
                                Some(color_index.get_color(state.bgp_register))
                            };
                        }
                        if wx_condition
                            && *wy_condition
                            && state.lcd_control.contains(LcdControl::WINDOW_ENABLE)
                        {
                            *internal_y_window_counter += 1;
                        }
                    }

                    // https://gbdev.io/pandocs/OAM.html#drawing-priority
                    // the objects that have the priority "BG over OBJ" enabled must override the other objects if
                    // their "normal" priority is higher
                    let mut obj_colors = [Option::<(Color, bool)>::None; 160];

                    if state.lcd_control.contains(LcdControl::OBJ_ENABLE) {
                        for (x, obj, color) in get_colors(
                            get_at_most_ten_objects_on_ly(work_state.ly, state),
                            work_state.ly,
                            state,
                        ) {
                            let x = usize::from(x);
                            if x < obj_colors.len() && color != ColorIndex::Zero {
                                obj_colors[x] = Some((
                                    color.get_color(
                                        if obj.flags.contains(ObjectFlags::DMG_PALETTE) {
                                            state.obp1
                                        } else {
                                            state.obp0
                                        },
                                    ),
                                    obj.flags.contains(ObjectFlags::PRIORITY),
                                ));
                            }
                        }
                    }

                    let bg_color = if state.lcd_control.contains(LcdControl::BG_AND_WINDOW_ENABLE) {
                        ColorIndex::Zero.get_color(state.bgp_register)
                    } else {
                        Color::White
                    };
                    for ((color, bg_win_color), obj_color) in
                        scanline.iter_mut().zip(bg_win_colors).zip(obj_colors)
                    {
                        *color = match (bg_win_color, obj_color) {
                            (None, None) => bg_color,
                            (None, Some((color, _))) | (_, Some((color, false))) => color,
                            (Some(color), Some((_, true)) | None) => color,
                        }
                    }
                }
                *dots_count += 1;
                if *dots_count == 172 {
                    mode_changed = true;
                    *self = Ppu::HorizontalBlank {
                        remaining_dots: NonZeroU8::new(204).unwrap(),
                        wy_condition: *wy_condition,
                        internal_y_window_counter: *internal_y_window_counter,
                    }
                }
            }
            Ppu::HorizontalBlank {
                remaining_dots,
                wy_condition,
                internal_y_window_counter,
            } => {
                if let Some(dots) = NonZeroU8::new(remaining_dots.get() - 1) {
                    *remaining_dots = dots;
                } else {
                    work_state.ly += 1;
                    mode_changed = true;
                    if work_state.ly == 144 {
                        *self = Ppu::VerticalBlankScanline {
                            remaining_dots: VERTICAL_BLANK_SCANLINE_DURATION,
                        };
                    } else {
                        *self = Ppu::OamScan {
                            remaining_dots: OAM_SCAN_DURATION,
                            wy_condition: *wy_condition,
                            internal_y_window_counter: *internal_y_window_counter,
                        }
                    }
                }
            }
            Ppu::VerticalBlankScanline { remaining_dots } => {
                if let Some(dots) = NonZeroU16::new(remaining_dots.get() - 1) {
                    *remaining_dots = dots;
                } else if work_state.ly == 153 {
                    mode_changed = true;
                    work_state.ly = 0;
                    *self = Default::default()
                } else {
                    work_state.ly += 1;
                    *self = Ppu::VerticalBlankScanline {
                        remaining_dots: VERTICAL_BLANK_SCANLINE_DURATION,
                    }
                }
            }
        };

        if !mode_changed {
            return;
        }

        let is_requesting_interrupt = match self {
            Ppu::OamScan { .. } => state.lcd_status.contains(LcdStatus::OAM_INT),
            Ppu::DrawingPixels { .. } => false,
            Ppu::HorizontalBlank { .. } => state.lcd_status.contains(LcdStatus::HBLANK_INT),
            Ppu::VerticalBlankScanline { .. } => state.lcd_status.contains(LcdStatus::VBLANK_INT),
        };

        work_state.is_requesting_lcd_int |= is_requesting_interrupt;
        work_state.is_requesting_vblank_int |= matches!(self, Ppu::VerticalBlankScanline { .. });
    }

    fn commit(&self, work_state: Self::WorkState, mut state: WriteOnlyState) {
        let mode = match self {
            Ppu::OamScan { .. } => LcdStatus::OAM_SCAN,
            Ppu::DrawingPixels { .. } => LcdStatus::DRAWING,
            Ppu::HorizontalBlank { .. } => LcdStatus::HBLANK,
            Ppu::VerticalBlankScanline { .. } => LcdStatus::VBLANK,
        };
        state.set_ppu_mode(mode);
        state.set_ly(work_state.ly);
        if work_state.is_requesting_lcd_int {
            state.insert_if(Ints::LCD);
        }
        if work_state.is_requesting_vblank_int {
            state.insert_if(Ints::VBLANK);
        }
    }
}

fn get_color_bg_win(
    scanline: Scanline,
    state: &State,
    internal_y_window_counter: Option<u8>,
) -> ColorIndex {
    let (picture_pixel, tile_map_address) = get_picture_pixel_and_tile_map_address(
        state.lcd_control,
        scanline,
        Window {
            x: state.wx,
            internal_y_window_counter,
        },
        Background {
            x: state.scx,
            y: state.scy,
        },
    );
    let tile_index = state.video_ram
        [usize::from(tile_map_address - VIDEO_RAM + picture_pixel.get_relative_tile_map_index())];
    let tile = get_bg_win_tile(
        state.video_ram[..0x1800].try_into().unwrap(),
        tile_index,
        !state.lcd_control.contains(LcdControl::BG_AND_WINDOW_TILES),
    );

    get_color_from_tile(tile, picture_pixel.x % 8, picture_pixel.y % 8)
}

// https://gbdev.io/pandocs/OAM.html#selection-priority
fn get_at_most_ten_objects_on_ly(ly: u8, state: &State) -> impl Iterator<Item = ObjectAttribute> {
    let is_big = state.lcd_control.contains(LcdControl::OBJ_SIZE);
    state
        .oam
        .chunks(4)
        .map(|slice| ObjectAttribute::from(<[u8; 4]>::try_from(slice).unwrap()))
        .filter(move |obj| obj.y <= ly + 16 && ly + 16 < (obj.y + if is_big { 16 } else { 8 }))
        .take(10)
}

// emit the color and the object priority flag for x on the current scanline. Can emit multiple time the same x.
// If multiple x emitted, the lastest takes priority. So don't think and write to the scanline in the same order as the
// x are emitted.
fn get_colors(
    objects_on_ly: impl IntoIterator<Item = ObjectAttribute>,
    ly: u8,
    state: &State,
) -> impl Iterator<Item = (u8, ObjectAttribute, ColorIndex)> {
    let is_big = state.lcd_control.contains(LcdControl::OBJ_SIZE);
    // Citation:
    // In Non-CGB mode, the smaller the X coordinate, the higher the priority.
    // When X coordinates are identical, the object located first in OAM has higher priority.
    let mut objects_on_ly: ArrayVec<_, 10> = objects_on_ly.into_iter().enumerate().collect();
    objects_on_ly.sort_unstable_by_key(|(index, obj)| (obj.x, *index));
    // rev to emit the most prioritary the lastest
    objects_on_ly.into_iter().rev().flat_map(move |(_, obj)| {
        // if is_big then the tile_index must be corrected to be always even
        // then we check if scanline.y reaches the second tile
        let y_flip = obj.flags.contains(ObjectFlags::Y_FLIP);
        let tile_index = (obj.tile_index & if is_big { 0xfe } else { 0xff })
            + (is_big && (ly + 8 >= obj.y) != y_flip) as u8;
        let tile = get_object_tile(
            state.video_ram[usize::from(0x8000 - VIDEO_RAM)..usize::from(0x9000 - VIDEO_RAM)]
                .try_into()
                .unwrap(),
            tile_index,
        );
        let mut y = (ly + 16 - obj.y) % 8;
        y = if y_flip { 7 - y } else { y };
        let line = get_line_from_tile(tile, y);
        (0..8).filter_map(move |x| {
            Some((
                (obj.x + x).checked_sub(8)?,
                obj,
                get_color_from_line(
                    line,
                    if obj.flags.contains(ObjectFlags::X_FLIP) {
                        7 - x
                    } else {
                        x
                    },
                ),
            ))
        })
    })
}

impl<T: StateMachine2> StateMachine for T {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        let mut work_state = T::get_work_state(state);
        self.execute(&mut work_state, state);
        Some(move |state: WriteOnlyState| self.commit(work_state, state))
    }
}

pub struct Speeder<T: StateMachine2>(pub T, pub NonZeroU8);

impl<T: StateMachine2> StateMachine for Speeder<T> {
    fn execute<'a>(&'a mut self, state: &State) -> Option<impl FnOnce(WriteOnlyState) + 'a> {
        let mut work_state = T::get_work_state(state);
        for _ in 0..self.1.get() {
            self.0.execute(&mut work_state, state);
        }
        Some(move |state: WriteOnlyState| {
            self.0.commit(work_state, state);
        })
    }
}
