mod background_fetcher;
pub mod color;
mod fifos;
mod renderer;
mod scanline;
mod sprite_fetcher;

use arrayvec::ArrayVec;

use crate::{
    WIDTH,
    ppu::renderer::Renderer,
    state::{Interruptions, LcdStatus, State},
};

pub use background_fetcher::get_bg_win_tile;
pub use scanline::Scanline;
pub use sprite_fetcher::get_line_from_tile;

#[derive(Clone)]
pub enum PpuStep {
    OamScan {
        dots_count: u8,
        // https://gbdev.io/pandocs/Scrolling.html#window
        window_y: Option<u8>,
        ly: u8,
    }, // <= 80
    Drawing {
        dots_count: u16,
        window_y: Option<u8>,
        renderer: Renderer,
        ly: u8,
    }, // <= 289
    HorizontalBlank {
        remaining_dots: u8,
        dots_count: u8,
        window_y: Option<u8>,
        scanline: Scanline,
        ly: u8,
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
    state: PpuState,
    previous_lyc: u8,
    // https://gbdev.io/pandocs/STAT#spurious-stat-interrupts
    queued_interrupt_part_lcd_status: Option<LcdStatus>,
    interrupt_part_lcd_status: LcdStatus,
    pub lyc: u8,
}

#[derive(Clone, Default)]
struct PpuState {
    lcd_control: LcdControl,
    bgp: u8,
    // OR effect on bgp change
    old_bgp: u8,
    old_lcd_control: LcdControl,
    old_old_lcd_control: LcdControl,
    scy: u8,
    scx: u8,
    wx: u8,
    old_wx: u8,
    old_old_wx: u8,
}

impl PpuState {
    pub fn get_effective_bgp(&self) -> u8 {
        self.bgp | self.old_bgp
    }

    pub fn refresh_old(&mut self) {
        self.old_bgp = self.bgp;
        self.old_old_lcd_control = self.old_lcd_control;
        self.old_lcd_control = self.lcd_control;
        self.old_old_wx = self.old_wx;
        self.old_wx = self.wx;
    }

    pub fn is_background_enabled(&self) -> bool {
        // there is a one dot delay when we disable the background
        // however, no delay when turning it back on
        (self.old_lcd_control | self.lcd_control).contains(LcdControl::BG_AND_WINDOW_ENABLE)
    }

    pub fn is_obj_enabled(&self) -> bool {
        self.old_lcd_control.contains(LcdControl::OBJ_ENABLE)
    }

    pub fn get_bg_tile_map_address(&self) -> u16 {
        if self.old_lcd_control.contains(LcdControl::BG_TILE_MAP) {
            0x9c00
        } else {
            0x9800
        }
    }

    pub fn is_signed_addressing(&self) -> bool {
        !self
            .old_lcd_control
            .contains(LcdControl::BG_AND_WINDOW_TILES)
    }

    pub fn get_window_tile_map_address(&self) -> u16 {
        if self.old_lcd_control.contains(LcdControl::WINDOW_TILE_MAP) {
            0x9c00
        } else {
            0x9800
        }
    }

    pub fn get_scrolling(&self) -> Scrolling {
        Scrolling {
            x: self.scx,
            y: self.scy,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Scrolling {
    // 0 < x < 256
    pub x: u8,
    // 0 < y < 256
    pub y: u8,
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

const OAM_SCAN_DURATION: u8 = 79;
const SCANLINE_DURATION: u16 = 456;
const VERTICAL_BLANK_DURATION: u16 = SCANLINE_DURATION * 10;

impl Default for PpuStep {
    fn default() -> Self {
        Self::OamScan {
            dots_count: 6,
            window_y: Default::default(),
            ly: 0,
        }
    }
}

// Tile data

const TILE_LENGTH: u8 = 16;

type TileVram = [u8; 0x1800];
type TileVramObj = [u8; 0x1000];
type Tile = [u8; 16];

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
    pub fn set_interrupt_part_lcd_status(&mut self, value: u8) {
        // https://www.devrs.com/gb/files/faqs.html#GBBugs
        // Citation: As far as has been figured out, the bug happens everytime
        // ANYTHING (including 00) is written to the STAT register ($ff41) while
        // the gameboy is either in HBLANK or VBLANK mode
        self.queued_interrupt_part_lcd_status = Some(LcdStatus::from_bits_truncate(value));
        // Citation: It behaves as if $FF were written for one M-cycle, and then the written value were written the next M-cycle
        self.interrupt_part_lcd_status = LcdStatus::from_bits_truncate(0xff)
    }

    pub fn get_ly(&self) -> u8 {
        match self.step {
            PpuStep::OamScan { ly, .. } => ly,
            PpuStep::Drawing { ly, .. } => ly,
            PpuStep::HorizontalBlank { ly, .. } => ly,
            PpuStep::VerticalBlankScanline { dots_count } => {
                if dots_count >= VERTICAL_BLANK_DURATION - SCANLINE_DURATION + 7 {
                    0
                } else {
                    144 + u8::try_from(dots_count / SCANLINE_DURATION).unwrap()
                }
            }
        }
    }

    pub fn get_lcd_status(&self) -> LcdStatus {
        let mut status =
            (self.interrupt_part_lcd_status & !LcdStatus::READONLY_MASK) | self.get_ppu_mode();
        status.set(LcdStatus::LYC_EQUAL_TO_LY, self.get_ly() == self.lyc);
        status
    }

    pub fn get_ppu_mode(&self) -> LcdStatus {
        if !self.is_ppu_enabled() {
            // https://gbdev.io/pandocs/STAT.html#ff41--stat-lcd-status
            // Citation: Reports 0 instead when the PPU is disabled.
            return LcdStatus::HBLANK;
        }

        use PpuStep::*;
        match self.step {
            OamScan { .. } => LcdStatus::OAM_SCAN,
            Drawing { .. } => LcdStatus::DRAWING,
            HorizontalBlank { .. } => LcdStatus::HBLANK,
            VerticalBlankScanline { .. } => LcdStatus::VBLANK,
        }
    }
    pub fn is_ppu_enabled(&self) -> bool {
        self.state.lcd_control.contains(LcdControl::LCD_PPU_ENABLE)
    }
    pub fn get_wx(&self) -> u8 {
        self.state.wx
    }
    pub fn set_wx(&mut self, value: u8) {
        self.state.wx = value;
    }
    pub fn get_scy(&self) -> u8 {
        self.state.scy
    }
    pub fn get_scx(&self) -> u8 {
        self.state.scx
    }
    pub fn set_scy(&mut self, value: u8) {
        self.state.scy = value;
    }
    pub fn set_scx(&mut self, value: u8) {
        self.state.scx = value;
    }
    pub fn get_bgp(&self) -> u8 {
        self.state.bgp
    }
    pub fn set_bgp(&mut self, bgp: u8) {
        self.state.bgp = bgp;
    }
    pub fn set_lcd_control(&mut self, new_control: LcdControl) {
        // if on -> off && not vblank
        if self.state.lcd_control.contains(LcdControl::LCD_PPU_ENABLE)
            && !new_control.contains(LcdControl::LCD_PPU_ENABLE)
            && !matches!(self.step, PpuStep::VerticalBlankScanline { .. })
        {
            // https://gbdev.io/pandocs/LCDC.html#lcdc7--lcd-enable
            log::warn!("LCD turned off outside of VBLANK (may damage hardware irl)")
        }
        // if off -> on
        if !self.state.lcd_control.contains(LcdControl::LCD_PPU_ENABLE)
            && new_control.contains(LcdControl::LCD_PPU_ENABLE)
        {
            self.step = Default::default();
        }
        self.state.lcd_control = new_control;
    }

    pub fn get_lcd_control(&self) -> LcdControl {
        self.state.lcd_control
    }

    #[must_use]
    pub fn get_scanline_if_ready(&self) -> Option<&Scanline> {
        match &self.step {
            PpuStep::HorizontalBlank {
                dots_count,
                remaining_dots,
                scanline,
                ..
            } if remaining_dots - dots_count < 4 => {
                // true once per scanline
                Some(scanline)
            }

            _ => None,
        }
    }

    fn switch_from_finished_mode(&mut self, state: &State, _: u64) {
        match &mut self.step {
            PpuStep::OamScan {
                window_y,
                dots_count: OAM_SCAN_DURATION,
                ly,
            } => {
                let mut objects_to_sort: ArrayVec<_, 10> = state
                    .oam
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .copied()
                    .map(ObjectAttribute::from)
                    .filter(|obj| {
                        let is_big = self.state.lcd_control.contains(LcdControl::OBJ_SIZE);
                        obj.y <= *ly + 16 && *ly + 16 < (obj.y + if is_big { 16 } else { 8 })
                    })
                    .take(10)
                    .enumerate()
                    .collect();
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
                );
                self.step = PpuStep::Drawing {
                    dots_count: 0,
                    renderer,
                    window_y: *window_y,
                    ly: *ly,
                }
            }
            PpuStep::Drawing {
                dots_count,
                renderer: Renderer { scanline, .. },
                window_y,
                ly,
                ..
            } if scanline.len() == WIDTH => {
                self.step = PpuStep::HorizontalBlank {
                    remaining_dots: u8::try_from(
                        SCANLINE_DURATION - u16::from(OAM_SCAN_DURATION) - *dots_count,
                    )
                    .unwrap(),
                    window_y: *window_y,
                    dots_count: 0,
                    scanline: *scanline.get_scanline(),
                    ly: *ly,
                }
            }
            PpuStep::HorizontalBlank {
                remaining_dots,
                window_y,
                dots_count,
                ly,
                ..
            } if remaining_dots == dots_count => {
                self.step = if *ly >= 143 {
                    PpuStep::VerticalBlankScanline { dots_count: 0 }
                } else {
                    PpuStep::OamScan {
                        window_y: *window_y,
                        dots_count: 0,
                        ly: *ly + 1,
                    }
                };
            }
            PpuStep::VerticalBlankScanline { dots_count, .. }
                if *dots_count == VERTICAL_BLANK_DURATION =>
            {
                self.step = PpuStep::OamScan {
                    window_y: Default::default(),
                    dots_count: 0,
                    ly: 0,
                }
            }
            _ => {}
        };
    }

    pub fn fire_interrupts(&mut self, state: &mut State, _: u64) {
        if let PpuStep::VerticalBlankScanline { dots_count: 2 } = self.step {
            state.interrupt_flag.insert(Interruptions::VBLANK);
        }

        let stat_mode_irq = match &self.step {
            PpuStep::OamScan { dots_count, ly, .. } => {
                // according to dmg schematics, mode 1 is "leaking" at the start of mode 2
                // on the first line
                let is_mode_1 = *ly == 0 && *dots_count < 2;
                is_mode_1
                    && self
                        .interrupt_part_lcd_status
                        .contains(LcdStatus::VBLANK_INT)
                    || !is_mode_1
                        && *dots_count < 4
                        && self.interrupt_part_lcd_status.contains(LcdStatus::OAM_INT)
            }
            PpuStep::HorizontalBlank {
                dots_count: 1.., ..
            } => self
                .interrupt_part_lcd_status
                .contains(LcdStatus::HBLANK_INT),
            PpuStep::VerticalBlankScanline { dots_count } => {
                *dots_count > 2
                    && self
                        .interrupt_part_lcd_status
                        .contains(LcdStatus::VBLANK_INT)
                    || *dots_count == 0
                        && self.interrupt_part_lcd_status.contains(LcdStatus::OAM_INT)
            }
            _ => false,
        };

        let lyc = self.interrupt_part_lcd_status.contains(LcdStatus::LYC_INT)
            && self.get_ly() == self.lyc;

        let old_lyc = self.previous_lyc & 0x04 != 0;
        self.previous_lyc <<= 1;
        self.previous_lyc |= lyc as u8;

        let stat_irq = stat_mode_irq || old_lyc;

        if stat_irq == self.stat_irq {
            return;
        }

        self.stat_irq = stat_irq;

        // rising edge described by https://raw.githubusercontent.com/geaz/emu-gameboy/master/docs/The%20Cycle-Accurate%20Game%20Boy%20Docs.pdf
        if stat_irq {
            state.interrupt_flag.insert(Interruptions::LCD);
        }
    }

    pub fn execute(&mut self, state: &mut State, cycles: u64) {
        if !self.is_ppu_enabled() {
            self.state.refresh_old();
            return;
        }
        self.switch_from_finished_mode(state, cycles);
        self.fire_interrupts(state, cycles);

        if let Some(value) = self.queued_interrupt_part_lcd_status.take() {
            self.interrupt_part_lcd_status = value;
        }

        // Pandocs says (https://gbdev.io/pandocs/Scrolling.html#window):
        // WY condition was triggered: i.e. at some point in this frame the value of WY was equal to LY (checked at the start of Mode 2 only)
        //
        // However, nothing in the schematics https://github.com/msinger/dmg-schematics/blob/2829269ee7cfbb681bc10deab6f3c1ee22c940e0/dmg_cpu_b/win_detect.kicad_sch
        // shows that the check is done at the start of Mode 2 (if my understanding is correct)
        match &mut self.step {
            PpuStep::OamScan { window_y, ly, .. }
            | PpuStep::Drawing { window_y, ly, .. }
            | PpuStep::HorizontalBlank { window_y, ly, .. }
                if window_y.is_none()
                    && self.state.lcd_control.contains(LcdControl::WINDOW_ENABLE)
                    && *ly == state.wy =>
            {
                *window_y = Some(0);
            }
            _ => {}
        }

        match &mut self.step {
            PpuStep::OamScan { dots_count, .. } => {
                *dots_count += 1;
            }
            PpuStep::Drawing {
                dots_count,
                renderer,
                window_y,
                ly,
                ..
            } => {
                renderer.execute(state, window_y, &self.state, *ly, cycles);

                *dots_count += 1;
            }
            PpuStep::HorizontalBlank { dots_count, .. } => *dots_count += 1,
            PpuStep::VerticalBlankScanline { dots_count } => *dots_count += 1,
        };

        self.state.refresh_old();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ppu::{LcdControl, Ppu, PpuStep, VERTICAL_BLANK_DURATION},
        state::State,
    };

    extern crate std;

    #[test]
    fn first_line_duration() {
        let mut ppu = Ppu::default();
        let mut state = State::default();
        ppu.set_lcd_control(LcdControl::LCD_PPU_ENABLE);
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
        let mut state = State::default();
        ppu.set_lcd_control(LcdControl::LCD_PPU_ENABLE);
        let mut duration = 0;
        loop {
            ppu.execute(&mut state, 0);
            duration += 1;
            if let PpuStep::VerticalBlankScanline {
                dots_count: VERTICAL_BLANK_DURATION,
                ..
            } = ppu.step
            {
                break;
            }
        }
        assert_eq!(70224, duration);
    }

    #[test]
    fn all_ly() {
        let mut ppu = Ppu::default();
        let mut state = State::default();
        ppu.set_lcd_control(LcdControl::LCD_PPU_ENABLE);
        let mut lys: std::collections::HashSet<_> = (0..154).collect();

        while !lys.is_empty() {
            ppu.execute(&mut state, 0);
            lys.remove(&ppu.get_ly());
        }
    }
}
