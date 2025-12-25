mod ly_handler;

use core::num::NonZeroU8;

use arrayvec::ArrayVec;

use crate::{
    StateMachine, WIDTH,
    ic::Ints,
    ppu::ly_handler::LyHandler,
    state::{LcdStatus, State, VIDEO_RAM},
};

// used by oam scan, drawing, hblank, discarded during vblank
#[derive(Clone, Copy, Default)]
pub struct CurrentFrameData {
    wy_condition: bool,
    internal_y_window_counter: u8,
}

#[derive(Clone)]
pub struct DrawingState {
    scanline: [Color; 160],
    // https://gbdev.io/pandocs/Scrolling.html#window
    wx_condition: bool,
    current_frame_data: CurrentFrameData,
    x: u8,
}

impl DrawingState {
    fn new(current_frame_data: CurrentFrameData) -> Self {
        Self {
            current_frame_data,
            scanline: [Color::Black; 160],
            wx_condition: false,
            x: 0,
        }
    }

    fn execute(&mut self, state: &State) {
        let mut bg_win_color = Option::<Color>::None;

        if state.lcd_control.contains(LcdControl::BG_AND_WINDOW_ENABLE) {
            // Citation:
            // the current X coordinate being rendered + 7 was equal to WX
            self.wx_condition |= self.x + 7 == state.wx;
            let scanline = Scanline {
                x: self.x,
                y: state.ly,
            };
            let color_index = get_color_bg_win(
                scanline,
                state,
                (self.wx_condition
                    && self.current_frame_data.wy_condition
                    && state.lcd_control.contains(LcdControl::WINDOW_ENABLE))
                .then_some(self.current_frame_data.internal_y_window_counter),
            );
            bg_win_color = if color_index == ColorIndex::Zero {
                None
            } else {
                Some(color_index.get_color(state.bgp_register))
            };
        }

        // https://gbdev.io/pandocs/OAM.html#drawing-priority
        // the objects that have the priority "BG over OBJ" enabled must override the other objects if
        // their "normal" priority is higher
        let mut obj_color = Option::<(Color, bool)>::None;

        if state.lcd_control.contains(LcdControl::OBJ_ENABLE) {
            // TODO maybe performance hit here
            for (incoming_x, obj, color) in get_colors(
                get_at_most_ten_objects_on_ly(state.ly, state),
                state.ly,
                state,
            ) {
                if incoming_x == self.x && color != ColorIndex::Zero {
                    obj_color = Some((
                        color.get_color(if obj.flags.contains(ObjectFlags::DMG_PALETTE) {
                            state.obp1
                        } else {
                            state.obp0
                        }),
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
        self.scanline[usize::from(self.x)] = match (bg_win_color, obj_color) {
            (None, None) => bg_color,
            (None, Some((color, _))) | (_, Some((color, false))) => color,
            (Some(color), Some((_, true)) | None) => color,
        };
        self.x += 1;
        if self.x == WIDTH
            && self.wx_condition
            && self.current_frame_data.wy_condition
            && state.lcd_control.contains(LcdControl::WINDOW_ENABLE)
        {
            self.current_frame_data.internal_y_window_counter += 1;
        }
    }
}

#[derive(Clone)]
pub enum Ppu {
    OamScan {
        remaining_dots: u8,
        // https://gbdev.io/pandocs/Scrolling.html#window
        current_frame_data: CurrentFrameData,
    }, // <= 80
    DrawingPixels {
        dots_count: u16,
        drawing_state: DrawingState,
    }, // <= 289
    HorizontalBlank {
        remaining_dots: u8,
        dots_count: u8,
        current_frame_data: CurrentFrameData,
        scanline: [Color; 160],
    }, // <= 204
    VerticalBlankScanline {
        remaining_dots: u16,
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

const OAM_SCAN_DURATION: u8 = 80;
const VERTICAL_BLANK_SCANLINE_DURATION: u16 = 456 * 10;

impl Default for Ppu {
    fn default() -> Self {
        Self::OamScan {
            remaining_dots: OAM_SCAN_DURATION,
            current_frame_data: Default::default(),
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

pub fn get_line_from_tile(tile: &Tile, y: u8) -> [u8; 2] {
    assert!(y < 8);
    tile[usize::from(y * 2)..usize::from((y + 1) * 2)]
        .try_into()
        .unwrap()
}

pub fn get_color_from_line(line: [u8; 2], x: u8) -> ColorIndex {
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
pub fn get_bg_win_tile(vram: &TileVram, index: u8, is_signed_addressing: bool) -> &Tile {
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

impl Ppu {
    #[must_use]
    pub fn get_scanline_if_ready(&self) -> Option<&[Color; 160]> {
        match self {
            // dots_count 0 is impossible to see from outside
            Self::HorizontalBlank {
                dots_count: ..5,
                scanline,
                ..
            } => Some(scanline),
            _ => None,
        }
    }

    fn switch_from_finished_mode(&mut self, ly: u8, cycle_count: u64) {
        match self {
            Ppu::OamScan {
                remaining_dots: 0,
                current_frame_data,
            } => {
                // log::warn!("{cycle_count}: Will draw on LY {}", state.ly);
                *self = Ppu::DrawingPixels {
                    dots_count: 0,
                    drawing_state: DrawingState::new(*current_frame_data),
                }
            }
            Ppu::DrawingPixels {
                dots_count,
                drawing_state:
                    DrawingState {
                        scanline,
                        current_frame_data,
                        x: WIDTH,
                        ..
                    },
            } => {
                *self = Ppu::HorizontalBlank {
                    remaining_dots: u8::try_from(376 - *dots_count).unwrap(),
                    current_frame_data: *current_frame_data,
                    dots_count: 0,
                    scanline: *scanline,
                }
            }
            Ppu::HorizontalBlank {
                remaining_dots,
                current_frame_data,
                dots_count,
                ..
            } if remaining_dots == dots_count => {
                *self = if ly == 144 {
                    // log::warn!("{cycle_count}: Entering vblank");
                    Ppu::VerticalBlankScanline {
                        remaining_dots: VERTICAL_BLANK_SCANLINE_DURATION,
                    }
                } else {
                    log::warn!("{cycle_count}: Entering oam scan");
                    Ppu::OamScan {
                        remaining_dots: OAM_SCAN_DURATION,
                        current_frame_data: *current_frame_data,
                    }
                };
            }
            Ppu::VerticalBlankScanline {
                remaining_dots: 0, ..
            } => {
                log::warn!("{cycle_count}: Entering oam scan");
                *self = Default::default()
            }
            _ => {}
        };
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
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

impl From<Color> for [u8; 4] {
    fn from(c: Color) -> Self {
        match c {
            Color::White => [0xff; 4],
            Color::LightGray => [0xaa, 0xaa, 0xaa, 0xff],
            Color::DarkGray => [0x55, 0x55, 0x55, 0xff],
            Color::Black => [0, 0, 0, 0xff],
        }
    }
}

fn request_interrupt(state: &mut State, mode_interrupt: LcdStatus, cycle_count: u64) {
    assert!(matches!(
        mode_interrupt,
        LcdStatus::HBLANK_INT | LcdStatus::OAM_INT | LcdStatus::VBLANK_INT
    ));
    if state.lcd_status.contains(mode_interrupt) {
        log::warn!("{cycle_count}: Requesting {mode_interrupt:?}");
        state.interrupt_flag.insert(Ints::LCD);
    }
}

// D'après "The cycle accurate gameboy docs":
// - Ly augmente de façon "indépendante". À la ligne 153, il ne vaut 153 que pendant le premier M-cycle ensuite il est tout de suite à 0.
// - Pour LYC, la comparaison est toujours fausse pendant le premier M-cycle d'une ligne et le troisième M-cycle de la ligne 153.
// - Le OAM scan commence seulement au deuxième M-cycle d'une ligne. En effet, les modes sont décalés par rapport à la ligne, le Hblank déborde
//  à la fin et est exécuté au premier M-cycle de la ligne prochaine. Cela implique qu'un même Hblank peut connaître deux valeurs de LY différentes.

// ce que veut mooneye: écart entre OAM_INT et STAT MODE HBLANK = 63 M-cycles ou 252 dots (80 + 172)
// cependant d'après "The cycle accurate gameboy docs", l'interruption de OAM_INT arrive un cycle plus tôt

// D'après un commentaire dans SameBoy: It seems that the STAT register's mode bits are always "late" by 4 T-cycles.
// Donc les modes ne sont pas décalés en fin de compte ?
// Supposons que les modes ne soient pas décalés mais que cela soit le STAT qui soit à la bourre.
// Cela expliquerait pourquoi l'interruption du Mode 2 arrive un cycle avant Stat=2 (sauf ligne 0)
// Or l'interruption vblank arrive toujours pile poil quand son stat passe à 1.
// Mooneye veut aussi que l'écart en OAM_INT et HBLANK_INT = 63 M-cycles
// Donc à partir de ces informations je peux conclure:
// - le OAM scan (mode 2) commence bien au cycle 0
// - son interruption est lancée cycle 0 (bien synchronisée) (cycle 1 à la ligne 0)
// - son STAT est en retard d'un M-cycle
// - Le Drawing (mode 3) commence bien au cycle 20, juste après OAM scan
// - son STAT est en retard d'un M-cycle
// - le Hblank (mode 0) commence après le Drawing de façon normale
// - son interruption est lancée dès le premier cycle (bien synchronisée)
// - son STAT n'est pas en retard, il est changé dès le premier cycle (bien synchronisé même cycle que l'interruption)
// - VBLANK a un retard d'un cycle sur son STAT et sur son interruption (j'en peux plus)
//
// Nouvelle info de Mooneye, il veut un écart parfait entre l'interruption de OAM scan et le changement de STAT en mode 3 (drawing)
// cependant actuellement le STAT du mode 3 est en retard, alors que l'interruption de OAM scan est parfait.
// Tout ça me donne l'impression que si le PPU actionne une interruption alors le CPU a un délai d'un cycle avant de le traiter.
// En effet, pour corriger ce timing Interruption Mode 2 => STAT Mode 3 il faudrait corriger le retard de STAT mode 3. Mais cela
// serait en contradiction avec "The cycle accurate gameboy docs" qui dit bien que le STAT Mode 3 a le retard.
// Donc on va tenter un délai d'un M-cycle pour le traitement d'une interruption de la part du CPU. Cela implique que le Hblank a
// aussi son STAT en retard, comme indiqué par le commentaire de SameBoy.
// De plus l'émulateur de mooneye a ce délai d'un M-cycle entre le PPU qui détecte une interruption et le traitement donc ça va dans ce sens.

// one iteration = one dot = (1/4 M-cyle DMG)
impl StateMachine for Ppu {
    fn execute(&mut self, state: &mut State, cycle_count: u64) {
        if !state.lcd_control.contains(LcdControl::LCD_PPU_ENABLE) {
            return;
        }

        self.switch_from_finished_mode(state.ly, cycle_count);

        match self {
            Ppu::OamScan {
                remaining_dots,
                current_frame_data,
            } => {
                const DELAYED: u8 = OAM_SCAN_DURATION - 4;
                // interruption is delayed only on line 0
                if matches!(
                    (*remaining_dots, state.ly),
                    (DELAYED, 0) | (OAM_SCAN_DURATION, 1..)
                ) {
                    request_interrupt(state, LcdStatus::OAM_INT, cycle_count)
                }
                // STAT delayed by one M-cycle
                if *remaining_dots == DELAYED {
                    state.set_ppu_mode(LcdStatus::OAM_SCAN);
                }
                // Citation:
                // at some point in this frame the value of WY was equal to LY (checked at the start of Mode 2 only)
                current_frame_data.wy_condition |=
                    *remaining_dots == OAM_SCAN_DURATION && state.ly == state.wy;
                *remaining_dots -= 1;
            }
            Ppu::DrawingPixels {
                dots_count,
                drawing_state,
            } => {
                // STAT delayed by one M-cycle
                if *dots_count == 4 {
                    state.set_ppu_mode(LcdStatus::DRAWING);
                }
                // https://gbdev.io/pandocs/Rendering.html#first12
                // https://gbdev.io/pandocs/Rendering.html#mode-3-length
                // rendering is paused for SCX % 8 dots
                if *dots_count >= 12 + u16::from(state.scx % 8) {
                    drawing_state.execute(state);
                }

                *dots_count += 1;
            }
            Ppu::HorizontalBlank { dots_count, .. } => {
                match *dots_count {
                    // we know from mooneye's hblank_ly_scx_timing-GS that if hblank is one dot late, then the interrupt is one
                    // whole M-cycle late. So I assume that the interrupt is triggered during the fourth dot of hblank
                    3 => request_interrupt(state, LcdStatus::HBLANK_INT, cycle_count),
                    4 => state.set_ppu_mode(LcdStatus::HBLANK),
                    _ => {}
                }
                *dots_count += 1
            }
            Ppu::VerticalBlankScanline { remaining_dots } => {
                if *remaining_dots == VERTICAL_BLANK_SCANLINE_DURATION - 4 {
                    request_interrupt(state, LcdStatus::VBLANK_INT, cycle_count);
                    state.interrupt_flag.insert(Ints::VBLANK);
                    state.set_ppu_mode(LcdStatus::VBLANK);
                }
                *remaining_dots -= 1;
            }
        };
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
        .as_chunks::<4>()
        .0
        .iter()
        .map(|slice| ObjectAttribute::from(*slice))
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

#[derive(Clone)]
pub struct Speeder<T: StateMachine>(pub T, pub NonZeroU8);

impl<T: StateMachine> StateMachine for Speeder<T> {
    fn execute(&mut self, state: &mut State, cycle_count: u64) {
        for _ in 0..self.1.get() {
            self.0.execute(state, cycle_count);
        }
    }
}

// separated systems because the ppu is faster than the other component and the behavior of each system is strange.
pub type PpuBundle = (LyHandler, Speeder<Ppu>);

pub fn get_ppu_bundle() -> PpuBundle {
    (
        LyHandler::default(),
        Speeder(Ppu::default(), NonZeroU8::new(4).unwrap()),
    )
}
