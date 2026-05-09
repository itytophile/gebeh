mod background_fetcher;
pub mod color;
mod color_palettes;
mod fifos;
pub mod hdma;
pub mod oam_dma;
pub mod renderer;
mod scanline;
pub mod sprite;
mod sprite_fetcher;
pub mod vram;

use crate::{
    Model, Ram, WIDTH,
    interrupts::Interrupts,
    mbc::Mbc,
    ppu::{
        oam_dma::{BLOCKED_OAM, Oam, OamDma},
        renderer::Renderer,
        scanline::ScanlineBuilder,
        sprite::Sprite,
    },
};

pub use background_fetcher::get_bg_win_tile;
pub use scanline::DmgScanline;
pub use sprite_fetcher::get_line_from_tile;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq, Default)]
    pub struct LcdStatus: u8 {
        const LYC_INT = 1 << 6;
        const OAM_INT = 1 << 5;
        const VBLANK_INT = 1 << 4;
        const HBLANK_INT = 1 << 3;
        const LYC_EQUAL_TO_LY = 1 << 2;
        // Drawing before ppu mask for debug output
        const DRAWING = 0b11;
        const PPU_MASK = 0b11;
        const HBLANK = 0;
        const VBLANK = 1;
        const OAM_SCAN = 0b10;
        const READONLY_MASK = 0b111;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
    pub struct TileAttributes: u8 {
        const PRIORITY = 1 << 7;
        const Y_FLIP = 1 << 6;
        const X_FLIP = 1 << 5;
        const DMG_PALETTE = 1 << 4;
        const CGB_BANK = 1 << 3;
    }
}

impl TileAttributes {
    pub fn get_cgb_palette_index(&self) -> u8 {
        self.bits() & 0x07
    }
}

#[derive(Clone)]
pub enum PpuStep<R: Renderer> {
    // only used on line 0 when the lcd has just turned on
    SkippedOamScan {
        dots_count: u8,
    },
    OamScan {
        dots_count: u8,
        // https://gbdev.io/pandocs/Scrolling.html#window
        window_y: Option<u8>,
        ly: u8,
    }, // <= 80
    Drawing {
        dots_count: u16,
        window_y: Option<u8>,
        renderer: R,
        ly: u8,
    }, // <= 289
    HorizontalBlank {
        remaining_dots: u8,
        dots_count: u8,
        window_y: Option<u8>,
        scanline: <<R as Renderer>::ScanlineBuilder as ScanlineBuilder>::Scanline,
        ly: u8,
    }, // <= 204
    VerticalBlankScanline {
        dots_count: u16,
        // no wy_condition because vblank means the frame ends
    }, // <= 456
}

#[derive(Clone)]
pub struct Ppu<M: Model> {
    pub step: PpuStep<M::Renderer>,
    stat_irq: bool,
    state: PpuState<<M::Renderer as Renderer>::Vram>,
    previous_lyc: u8,
    stat_register_handler: M::StatRegisterHandler,
    interrupt_part_lcd_status: LcdStatus,
    pub lyc: u8,
    oam_dma: OamDma,
    extra: <M::Renderer as Renderer>::Extra,
}

impl<M: Model> Default for Ppu<M> {
    fn default() -> Self {
        Self {
            step: Default::default(),
            stat_irq: false,
            state: Default::default(),
            previous_lyc: 0,
            stat_register_handler: M::StatRegisterHandler::default(),
            interrupt_part_lcd_status: LcdStatus::default(),
            lyc: 0,
            oam_dma: Default::default(),
            extra: Default::default(),
        }
    }
}

#[derive(Clone, Default)]
pub struct PpuState<V> {
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
    video_ram: V,
    obp0: u8,
    obp1: u8,
    wy: u8,
}

impl<V> PpuState<V> {
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

impl<R: Renderer> Default for PpuStep<R> {
    fn default() -> Self {
        Self::SkippedOamScan { dots_count: 2 }
    }
}

// Tile data

const TILE_LENGTH: u8 = 16;

type TileVram = [u8; 0x1800];
type TileVramObj = [u8; 0x1000];
type Tile = [u8; 16];

pub trait StatRegisterHandler: Default + Clone {
    fn set_interrupt_part_lcd_status(&mut self, value: u8, stat_reg: &mut LcdStatus);
    fn after_interrupt_handling(&mut self, stat_reg: &mut LcdStatus);
}

#[derive(Default, Clone)]
pub struct StatInterruptWriteQuirk {
    // https://gbdev.io/pandocs/STAT#spurious-stat-interrupts
    queued_interrupt_part_lcd_status: Option<LcdStatus>,
}

impl StatRegisterHandler for StatInterruptWriteQuirk {
    fn set_interrupt_part_lcd_status(&mut self, value: u8, stat_reg: &mut LcdStatus) {
        // https://www.devrs.com/gb/files/faqs.html#GBBugs
        // Citation: As far as has been figured out, the bug happens everytime
        // ANYTHING (including 00) is written to the STAT register ($ff41) while
        // the gameboy is either in HBLANK or VBLANK mode
        self.queued_interrupt_part_lcd_status = Some(LcdStatus::from_bits_truncate(value));
        // Citation: It behaves as if $FF were written for one M-cycle, and then the written value were written the next M-cycle
        *stat_reg = LcdStatus::from_bits_truncate(0xff)
    }

    fn after_interrupt_handling(&mut self, stat_reg: &mut LcdStatus) {
        if let Some(value) = self.queued_interrupt_part_lcd_status.take() {
            *stat_reg = value;
        }
    }
}

impl StatRegisterHandler for () {
    fn set_interrupt_part_lcd_status(&mut self, value: u8, stat_reg: &mut LcdStatus) {
        *stat_reg = LcdStatus::from_bits_truncate(value);
    }
    fn after_interrupt_handling(&mut self, _: &mut LcdStatus) {}
}

// one iteration = one dot = (1/4 M-cyle DMG)
impl<M: Model> Ppu<M> {
    pub fn trigger_dma(&mut self, value: u8) {
        self.oam_dma.trigger_dma(value);
    }
    pub fn execute_dma(&mut self, mbc: &(impl Mbc + ?Sized), wram: &impl Ram, cycles: u64) {
        self.oam_dma.execute(
            mbc,
            if self.get_ppu_mode() != LcdStatus::DRAWING {
                Some(&self.state.video_ram)
            } else {
                None
            },
            wram,
            cycles,
        );
    }
    pub fn get_dma_register(&self) -> u8 {
        self.oam_dma.dma_register
    }
    pub fn get_oam(&self) -> &Oam {
        let mode = self.get_ppu_mode();
        if mode == LcdStatus::OAM_SCAN || mode == LcdStatus::DRAWING {
            &BLOCKED_OAM
        } else {
            self.oam_dma.get_oam()
        }
    }
    pub fn write_oam(&mut self, index: u8, value: u8) {
        let mode = self.get_ppu_mode();
        if mode == LcdStatus::OAM_SCAN || mode == LcdStatus::DRAWING {
            return;
        }
        self.oam_dma.write_oam(index, value);
    }
    pub fn get_vram_if_available(&self) -> Option<&<M::Renderer as Renderer>::Vram> {
        if self.get_ppu_mode() == LcdStatus::DRAWING {
            None
        } else {
            Some(&self.state.video_ram)
        }
    }
    pub fn get_wy(&self) -> u8 {
        self.state.wy
    }
    pub fn set_wy(&mut self, value: u8) {
        self.state.wy = value;
    }
    pub fn get_obp0(&self) -> u8 {
        self.state.obp0
    }
    pub fn set_obp0(&mut self, value: u8) {
        self.state.obp0 = value
    }
    pub fn get_obp1(&self) -> u8 {
        self.state.obp1
    }
    pub fn set_obp1(&mut self, value: u8) {
        self.state.obp1 = value
    }
    pub fn get_vram(&self) -> &<M::Renderer as Renderer>::Vram {
        &self.state.video_ram
    }
    pub fn get_vram_mut(&mut self) -> &mut <M::Renderer as Renderer>::Vram {
        &mut self.state.video_ram
    }
    pub fn write_vram(&mut self, index: u16, value: u8) {
        if self.get_ppu_mode() != LcdStatus::DRAWING {
            self.state.video_ram[usize::from(index)] = value;
        }
    }
    pub fn set_interrupt_part_lcd_status(&mut self, value: u8) {
        self.stat_register_handler
            .set_interrupt_part_lcd_status(value, &mut self.interrupt_part_lcd_status);
    }

    pub fn get_ly(&self) -> u8 {
        match self.step {
            PpuStep::SkippedOamScan { .. } => 0,
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
            PpuStep::SkippedOamScan { .. } => LcdStatus::HBLANK,
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
    pub fn get_scanline_if_ready(
        &self,
    ) -> Option<&<<M::Renderer as Renderer>::ScanlineBuilder as ScanlineBuilder>::Scanline> {
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

    fn switch_from_finished_mode(&mut self, _: u64) {
        match &mut self.step {
            PpuStep::SkippedOamScan {
                dots_count: OAM_SCAN_DURATION,
            } => {
                self.step = PpuStep::Drawing {
                    dots_count: 0,
                    renderer: M::Renderer::new(Default::default()),
                    window_y: None,
                    ly: 0,
                }
            }
            PpuStep::OamScan {
                window_y,
                dots_count: OAM_SCAN_DURATION,
                ly,
            } => {
                let renderer = M::Renderer::new(M::parse_objects(
                    self.oam_dma.get_oam(),
                    self.state.lcd_control,
                    *ly,
                ));
                self.step = PpuStep::Drawing {
                    dots_count: 0,
                    renderer,
                    window_y: *window_y,
                    ly: *ly,
                }
            }
            PpuStep::Drawing {
                dots_count,
                renderer,
                window_y,
                ly,
                ..
            } if renderer.get_scanline_builder().len() == WIDTH => {
                self.step = PpuStep::HorizontalBlank {
                    remaining_dots: u8::try_from(
                        SCANLINE_DURATION - u16::from(OAM_SCAN_DURATION) - *dots_count,
                    )
                    .unwrap(),
                    window_y: *window_y,
                    dots_count: 0,
                    scanline: renderer.get_scanline_builder().get_scanline().clone(),
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

    pub fn fire_interrupts(&mut self, interrupts: &mut Interrupts, _: u64) {
        if let PpuStep::VerticalBlankScanline { dots_count: 2 } = self.step {
            interrupts.insert(Interrupts::VBLANK);
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
            interrupts.insert(Interrupts::LCD);
        }
    }

    pub fn execute(&mut self, interrupts: &mut Interrupts, cycles: u64) {
        if !self.is_ppu_enabled() {
            self.state.refresh_old();
            return;
        }
        self.switch_from_finished_mode(cycles);
        self.fire_interrupts(interrupts, cycles);

        self.stat_register_handler
            .after_interrupt_handling(&mut self.interrupt_part_lcd_status);

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
                    && *ly == self.state.wy =>
            {
                *window_y = Some(0);
            }
            _ => {}
        }

        match &mut self.step {
            PpuStep::SkippedOamScan { dots_count }
            | PpuStep::OamScan { dots_count, .. }
            | PpuStep::HorizontalBlank { dots_count, .. } => {
                *dots_count += 1;
            }
            PpuStep::Drawing {
                dots_count,
                renderer,
                window_y,
                ly,
                ..
            } => {
                renderer.execute(window_y, &self.state, &self.extra, *ly, cycles);

                *dots_count += 1;
            }
            PpuStep::VerticalBlankScanline { dots_count } => *dots_count += 1,
        };

        self.state.refresh_old();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Dmg,
        interrupts::Interrupts,
        ppu::{LcdControl, Ppu, PpuStep},
    };

    extern crate std;

    #[test]
    fn line_duration() {
        let mut ppu = Ppu::<Dmg>::default();
        let mut interrupts = Interrupts::default();
        ppu.set_lcd_control(LcdControl::LCD_PPU_ENABLE);
        // to ignore SkippedOamScan when the ppu is turning on
        loop {
            if let PpuStep::OamScan { .. } = ppu.step {
                break;
            }
            ppu.execute(&mut interrupts, 0);
        }
        let mut duration = 0;
        let current_ly = ppu.get_ly();
        while ppu.get_ly() <= current_ly {
            duration += 1;
            ppu.execute(&mut interrupts, 0);
        }
        assert_eq!(456, duration);
    }

    #[test]
    fn frame_duration() {
        let mut ppu = Ppu::<Dmg>::default();
        let mut interrupts = Interrupts::default();
        ppu.set_lcd_control(LcdControl::LCD_PPU_ENABLE);
        // to ignore SkippedOamScan when the ppu is turning on
        loop {
            if let PpuStep::OamScan {
                ly: 0,
                dots_count: 1,
                ..
            } = ppu.step
            {
                break;
            }
            ppu.execute(&mut interrupts, 0);
        }
        let mut duration = 1;
        ppu.execute(&mut interrupts, 0);
        loop {
            if let PpuStep::OamScan {
                ly: 0,
                dots_count: 1,
                ..
            } = ppu.step
            {
                break;
            }
            ppu.execute(&mut interrupts, 0);
            duration += 1;
        }
        assert_eq!(70224, duration);
    }

    #[test]
    fn all_ly() {
        let mut ppu = Ppu::<Dmg>::default();
        let mut interrupts = Interrupts::default();
        ppu.set_lcd_control(LcdControl::LCD_PPU_ENABLE);
        let mut lys: std::collections::HashSet<_> = (0..154).collect();

        while !lys.is_empty() {
            ppu.execute(&mut interrupts, 0);
            lys.remove(&ppu.get_ly());
        }
    }
}
