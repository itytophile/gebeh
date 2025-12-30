mod background_fetcher;
mod fifos;
mod ly_handler;
mod renderer;
mod sprite_fetcher;

use core::num::NonZeroU8;

use arrayvec::ArrayVec;

use crate::{
    ppu::renderer::Renderer,
    state::{Interruptions, LcdStatus, State},
};

pub use ly_handler::LyHandler;

#[derive(Clone)]
pub enum PpuStep {
    OamScan {
        dots_count: u8,
        // https://gbdev.io/pandocs/Scrolling.html#window
        window_y: Option<u8>,
        objects: ArrayVec<ObjectAttribute, 10>,
    }, // <= 80
    Drawing {
        dots_count: u16,
        window_y: Option<u8>,
        renderer: Renderer,
    }, // <= 289
    HorizontalBlank {
        remaining_dots: u8,
        dots_count: u8,
        window_y: Option<u8>,
        scanline: [Color; 160],
    }, // <= 204
    VerticalBlankScanline {
        dots_count: u16,
        // no wy_condition because vblank means the frame ends
    }, // <= 456
}

#[derive(Clone, Default)]
pub struct Ppu {
    pub step: PpuStep,
    stat_irq: bool,
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

impl LcdControl {
    pub fn get_bg_tile_map_address(self) -> u16 {
        if self.contains(LcdControl::BG_TILE_MAP) {
            0x9c00
        } else {
            0x9800
        }
    }

    pub fn get_window_tile_map_address(self) -> u16 {
        if self.contains(LcdControl::WINDOW_TILE_MAP) {
            0x9c00
        } else {
            0x9800
        }
    }
}

const OAM_SCAN_DURATION: u8 = 80;
const VERTICAL_BLANK_SCANLINE_DURATION: u16 = 456 * 10;

impl Default for PpuStep {
    fn default() -> Self {
        Self::OamScan {
            dots_count: 0,
            window_y: Default::default(),
            objects: Default::default(),
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
pub struct ObjectAttribute {
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

// TODO if the PPU’s access to VRAM is blocked then the tile data is read as $FF

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
impl Ppu {
    #[must_use]
    pub fn get_scanline_if_ready(&self) -> Option<&[Color; 160]> {
        match &self.step {
            PpuStep::HorizontalBlank {
                dots_count,
                remaining_dots,
                scanline,
                ..
            } if dots_count == remaining_dots => Some(scanline),
            _ => None,
        }
    }

    fn switch_from_finished_mode(&mut self, state: &State) {
        match &mut self.step {
            PpuStep::OamScan {
                window_y,
                dots_count: OAM_SCAN_DURATION,
                objects,
            } => {
                let mut objects_to_sort: ArrayVec<_, 10> =
                    objects.iter().copied().enumerate().collect();
                // https://gbdev.io/pandocs/OAM.html#drawing-priority
                // Citation: the smaller the X coordinate, the higher the priority.
                // When X coordinates are identical, the object located first in OAM has higher priority.
                objects_to_sort.sort_unstable_by_key(|(index, obj)| (obj.x, *index));
                let renderer = Renderer::new(
                    objects_to_sort
                        .into_iter()
                        .rev() // because we will pop the objects
                        .map(|(_, object)| object)
                        .collect(),
                    // https://gbdev.io/pandocs/Scrolling.html#scrolling
                    // Citation: The scroll registers are re-read on each tile fetch, except for
                    // the low 3 bits of SCX, which are only read at the beginning of the scanline
                    // (I have a visual glitch on the OH demo when I set this at the start of OAM scan so I'll do it here instead)
                    // EDIT: i'm still one cycle late
                    state.scx,
                );

                // log::warn!("{cycles}: entering drawing with ly = {}", state.ly);

                self.step = PpuStep::Drawing {
                    dots_count: 0,
                    renderer,
                    window_y: *window_y,
                }
            }
            PpuStep::Drawing {
                dots_count,
                renderer: Renderer { scanline, .. },
                window_y,
                ..
            } => {
                if let Ok(scanline) = scanline.as_slice().try_into() {
                    self.step = PpuStep::HorizontalBlank {
                        remaining_dots: u8::try_from(376 - *dots_count).unwrap(),
                        window_y: *window_y,
                        dots_count: 0,
                        scanline,
                    }
                }
            }
            PpuStep::HorizontalBlank {
                remaining_dots,
                window_y,
                dots_count,
                ..
            } if remaining_dots == dots_count => {
                self.step = if state.ly == 144 {
                    PpuStep::VerticalBlankScanline { dots_count: 0 }
                } else {
                    PpuStep::OamScan {
                        window_y: *window_y,
                        dots_count: 0,
                        objects: Default::default(),
                    }
                };
            }
            PpuStep::VerticalBlankScanline {
                dots_count: VERTICAL_BLANK_SCANLINE_DURATION,
                ..
            } => {
                self.step = PpuStep::OamScan {
                    window_y: Default::default(),
                    dots_count: 0,
                    objects: Default::default(),
                }
            }
            _ => {}
        };
    }

    pub fn detect_stat_irq(&mut self, state: &mut State) {
        let stat_mode_irq = match &self.step {
            PpuStep::OamScan { .. } => state.lcd_status.contains(LcdStatus::OAM_INT),
            PpuStep::HorizontalBlank { .. } => state.lcd_status.contains(LcdStatus::HBLANK_INT),
            PpuStep::VerticalBlankScanline { .. } => {
                // according to https://github.com/Gekkio/mooneye-test-suite/blob/443f6e1f2a8d83ad9da051cbb960311c5aaaea66/acceptance/ppu/vblank_stat_intr-GS.s
                state.lcd_status.contains(LcdStatus::OAM_INT)
                    | state.lcd_status.contains(LcdStatus::VBLANK_INT)
            }
            _ => false,
        };
        let stat_irq = stat_mode_irq
            || (state.lcd_status.contains(LcdStatus::LYC_INT) && state.ly == state.lyc);

        if stat_irq == self.stat_irq {
            return;
        }

        self.stat_irq = stat_irq;

        // rising edge described by https://raw.githubusercontent.com/geaz/emu-gameboy/master/docs/The%20Cycle-Accurate%20Game%20Boy%20Docs.pdf
        if stat_irq {
            state.interrupt_flag.insert(Interruptions::LCD);
        }
    }

    pub fn execute(&mut self, state: &mut State, _: u64) {
        if !state.lcd_control.contains(LcdControl::LCD_PPU_ENABLE) {
            return;
        }

        self.switch_from_finished_mode(state);
        self.detect_stat_irq(state);

        match &mut self.step {
            PpuStep::OamScan {
                dots_count,
                window_y,
                objects,
                ..
            } => {
                // STAT delayed by one M-cycle
                if *dots_count == 4 {
                    state.set_ppu_mode(LcdStatus::OAM_SCAN);
                }

                if state.lcd_control.contains(LcdControl::OBJ_ENABLE)
                    && *dots_count % 2 == 0
                    && objects.len() < objects.capacity()
                {
                    let base = usize::from(*dots_count * 2);
                    let obj = ObjectAttribute::from(
                        <[u8; 4]>::try_from(&state.oam[base..base + 4]).unwrap(),
                    );
                    let is_big = state.lcd_control.contains(LcdControl::OBJ_SIZE);
                    if obj.y <= state.ly + 16
                        && state.ly + 16 < (obj.y + if is_big { 16 } else { 8 })
                    {
                        objects.push(obj);
                    }
                }

                // Citation:
                // at some point in this frame the value of WY was equal to LY (checked at the start of Mode 2 only)
                if window_y.is_none() && *dots_count == 0 && state.ly == state.wy {
                    *window_y = Some(0);
                }
                *dots_count += 1;
            }
            PpuStep::Drawing {
                dots_count,
                renderer,
                window_y,
                ..
            } => {
                // STAT delayed by one M-cycle
                if *dots_count == 4 {
                    state.set_ppu_mode(LcdStatus::DRAWING);
                    // hot fix, we sometimes miss the scx value by one M-cycle
                    // so we reset the renderer here
                    *renderer = Renderer::new(core::mem::take(&mut renderer.objects), state.scx);
                    // don't forget that the renderer takes 174 dots to render a screen (minimum) so we must
                    // run it two times more
                    for _ in 0..6 {
                        renderer.execute(state, *dots_count, window_y);
                    }
                }

                if *dots_count >= 4 {
                    renderer.execute(state, *dots_count, window_y);
                }

                *dots_count += 1;
            }
            PpuStep::HorizontalBlank { dots_count, .. } => {
                if *dots_count == 4 {
                    state.set_ppu_mode(LcdStatus::HBLANK)
                }

                *dots_count += 1
            }
            PpuStep::VerticalBlankScanline { dots_count } => {
                if *dots_count == 4 {
                    state.interrupt_flag.insert(Interruptions::VBLANK);
                    state.set_ppu_mode(LcdStatus::VBLANK);
                }
                *dots_count += 1;
            }
        };
    }
}

#[derive(Clone)]
pub struct Speeder(pub Ppu, pub NonZeroU8);

impl Speeder {
    pub fn execute(&mut self, state: &mut State, cycle_count: u64) {
        for _ in 0..self.1.get() {
            self.0.execute(state, cycle_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ppu::{LcdControl, LyHandler, Ppu, PpuStep, VERTICAL_BLANK_SCANLINE_DURATION},
        state::State,
    };

    #[test]
    fn first_line_duration() {
        let mut ppu = Ppu::default();
        let mut state = State::default();
        state.lcd_control.insert(LcdControl::LCD_PPU_ENABLE);
        let mut duration = 0;
        // we don't count this iteration, it's to skip the first Ppu::OamScan { dots_count: 1 }
        ppu.execute(&mut state, 0);
        loop {
            ppu.execute(&mut state, 0);
            duration += 1;
            if let PpuStep::OamScan { dots_count: 1, .. } = ppu.step {
                break;
            }
        }
        assert_eq!(456, duration);
    }

    #[test]
    fn frame_duration() {
        let mut ppu = Ppu::default();
        let mut ly_handler = LyHandler::default();
        let mut state = State::default();
        state.lcd_control.insert(LcdControl::LCD_PPU_ENABLE);
        let mut duration = 0;
        loop {
            if duration % 4 == 0 {
                ly_handler.execute(&mut state, 0);
            }

            ppu.execute(&mut state, 0);
            duration += 1;
            if let PpuStep::VerticalBlankScanline {
                dots_count: VERTICAL_BLANK_SCANLINE_DURATION,
                ..
            } = ppu.step
            {
                break;
            }
        }
        assert_eq!(70224, duration);
    }
}
