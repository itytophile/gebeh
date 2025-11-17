use core::convert::TryInto;

use crate::hardware::{VRAM_HEIGHT, VRAM_WIDTH};
use crate::ic::{Ints, Irq};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq, Default)]
    struct LcdStatus: u8 {
        const LYC_INT = 1 << 6;
        const OAM_INT = 1 << 5;
        const VBLANK_INT = 1 << 4;
        const HBLANK_INT = 1 << 3;
    }
}

// https://gbdev.io/pandocs/Rendering.html#ppu-modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    OamScan,
    Drawing,
    HBlank,
    VBlank,
    None,
}

impl From<Mode> for u8 {
    fn from(v: Mode) -> u8 {
        match v {
            Mode::HBlank => 0,
            Mode::VBlank => 1,
            Mode::OamScan => 2,
            Mode::Drawing => 3,
            Mode::None => 0,
        }
    }
}

impl From<u8> for Mode {
    fn from(v: u8) -> Mode {
        match v {
            0 => Mode::HBlank,
            1 => Mode::VBlank,
            2 => Mode::OamScan,
            3 => Mode::Drawing,
            _ => Mode::None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Point {
    x: u8,
    y: u8,
}

#[derive(Copy, Clone)]
pub struct Dmg {
    pub bg_palette: [DmgColor; 4],
    pub obj_palette0: [DmgColor; 4],
    pub obj_palette1: [DmgColor; 4],
}

pub fn to_palette(p: u8) -> [DmgColor; 4] {
    [
        (p & 0x3).into(),
        ((p >> 2) & 0x3).into(),
        ((p >> 4) & 0x3).into(),
        ((p >> 6) & 0x3).into(),
    ]
}

impl Default for Dmg {
    fn default() -> Self {
        Self {
            bg_palette: [
                DmgColor::White,
                DmgColor::LightGray,
                DmgColor::DarkGray,
                DmgColor::Black,
            ],
            obj_palette0: [
                DmgColor::White,
                DmgColor::LightGray,
                DmgColor::DarkGray,
                DmgColor::Black,
            ],
            obj_palette1: [
                DmgColor::White,
                DmgColor::LightGray,
                DmgColor::DarkGray,
                DmgColor::Black,
            ],
        }
    }
}

impl Dmg {
    fn get_sp_attr(&self, attr: u8) -> MapAttribute {
        let palette = if attr & 0x10 != 0 {
            self.obj_palette1
        } else {
            self.obj_palette0
        };

        MapAttribute {
            palette,
            xflip: attr & 0x20 != 0,
            yflip: attr & 0x40 != 0,
            priority: attr & 0x80 != 0,
        }
    }

    fn get_scanline_after_offset(
        &self,
        scx: u8,
        y: u8,
        vram_bank0: &[u8; 0x2000],
        tiles: u16,
        mapbase: u16,
        buf: &mut [DmgColor; VRAM_WIDTH as usize],
        mut bgbuf: Option<&mut [u8; VRAM_WIDTH as usize]>,
    ) {
        // thanks https://github.com/deltabeard/Peanut-GB/blob/4596d56ddb85a1aa45b1197c77f05e236a23bd94/peanut_gb.h#L1465
        let mut tbase = get_tile_base(
            tiles,
            mapbase,
            Point {
                x: (VRAM_WIDTH - 1).wrapping_add(scx) / 8,
                y: y / 8,
            },
            vram_bank0,
        );
        let mut line = get_tile_line(tbase, y % 8, vram_bank0);
        let mut offset = (8 - (scx % 8)) % 8;
        line[0] >>= offset;
        line[1] >>= offset;
        for i in (0..VRAM_WIDTH).rev() {
            if offset == 8 {
                tbase = get_tile_base(
                    tiles,
                    mapbase,
                    Point {
                        x: i.wrapping_add(scx) / 8,
                        y: y / 8,
                    },
                    vram_bank0,
                );
                line = get_tile_line(tbase, y % 8, vram_bank0);
                offset = 0;
            }
            let coli = (line[0] & 1) | ((line[1] & 1) << 1);
            buf[usize::from(i)] = self.bg_palette[usize::from(coli)];
            if let Some(b) = bgbuf {
                b[usize::from(i)] = coli;
                bgbuf = Some(b);
            }

            line[0] >>= 1;
            line[1] >>= 1;
            offset += 1;
        }
    }
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
    fn get_bgmap(self) -> u16 {
        if self.contains(Self::BG_TILE_MAP) {
            0x9c00
        } else {
            0x9800
        }
    }

    fn get_bg_and_window_tile_area(self) -> u16 {
        if self.contains(Self::BG_AND_WINDOW_TILES) {
            0x8000
        } else {
            0x8800
        }
    }

    fn get_winmap(self) -> u16 {
        if self.contains(Self::WINDOW_TILE_MAP) {
            0x9c00
        } else {
            0x9800
        }
    }

    fn get_spsize(self) -> u8 {
        if self.contains(Self::OBJ_SIZE) { 16 } else { 8 }
    }
}

pub struct Gpu {
    clocks: usize,

    lcd_status: LcdStatus,
    mode: Mode,

    lyc: u8,

    wx: u8,
    wy: u8,

    oam: [u8; 0xa0],

    // useful to keep the data here to avoid the CPU struct to catch some generic bounds
    pub draw_line: [DmgColor; VRAM_WIDTH as usize],
    lcd_control: LcdControl,
}

pub struct MapAttribute {
    palette: [DmgColor; 4],
    xflip: bool,
    yflip: bool,
    priority: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DmgColor {
    #[default]
    White,
    LightGray,
    DarkGray,
    Black,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Dmg(DmgColor),
    Rgb(u8, u8, u8),
}

impl Default for Color {
    fn default() -> Self {
        Color::Dmg(DmgColor::default())
    }
}

fn color_adjust(v: u8) -> u32 {
    let v = u32::from(v);

    if v >= 0x10 { 0xff - (0x1f - v) } else { v }
}

impl From<Color> for u32 {
    fn from(c: Color) -> u32 {
        match c {
            Color::Dmg(dmg) => u32::from(dmg),
            Color::Rgb(r, g, b) => {
                let mut c = 0;
                c |= color_adjust(r) << 16;
                c |= color_adjust(g) << 8;
                c |= color_adjust(b);
                c
            }
        }
    }
}

impl From<DmgColor> for u32 {
    fn from(c: DmgColor) -> u32 {
        match c {
            DmgColor::White => 0xffffff,
            DmgColor::LightGray => 0xaaaaaa,
            DmgColor::DarkGray => 0x555555,
            DmgColor::Black => 0x000000,
        }
    }
}

impl From<Color> for u8 {
    fn from(c: Color) -> u8 {
        match c {
            Color::Dmg(dmg) => u8::from(dmg),
            _ => unreachable!(),
        }
    }
}

impl From<DmgColor> for u8 {
    fn from(c: DmgColor) -> u8 {
        match c {
            DmgColor::White => 0,
            DmgColor::LightGray => 1,
            DmgColor::DarkGray => 2,
            DmgColor::Black => 3,
        }
    }
}

impl From<u8> for Color {
    fn from(v: u8) -> Color {
        Color::Dmg(v.into())
    }
}

impl From<u8> for DmgColor {
    fn from(v: u8) -> DmgColor {
        match v {
            0 => DmgColor::White,
            1 => DmgColor::LightGray,
            2 => DmgColor::DarkGray,
            3 => DmgColor::Black,
            _ => unreachable!(),
        }
    }
}

impl Default for Gpu {
    fn default() -> Self {
        Self {
            clocks: 0,
            lcd_status: LcdStatus::empty(),
            mode: Mode::None,
            lyc: 0,
            wx: 0,
            wy: 0,

            oam: [0; 0xa0],
            draw_line: [Default::default(); VRAM_WIDTH as usize],
            lcd_control: LcdControl::empty(),
        }
    }
}

impl Gpu {
    pub(crate) fn write_ctrl(&mut self, value: LcdControl, irq: &mut Irq) {
        if !self.lcd_control.contains(LcdControl::LCD_PPU_ENABLE)
            && value.contains(LcdControl::LCD_PPU_ENABLE)
        {
            self.clocks = 0;
            self.mode = Mode::HBlank;
        } else if self.lcd_control.contains(LcdControl::LCD_PPU_ENABLE)
            && !value.contains(LcdControl::LCD_PPU_ENABLE)
        {
            self.mode = Mode::None;
        }

        self.lcd_control = value;

        irq.request.remove(Ints::VBLANK);
    }

    pub fn step(
        &mut self,
        time: usize,
        mut irq: Irq,
        mut ly: u8,
        lcd_control: LcdControl,
        scx: u8,
        scy: u8,
        vram: &[u8; 0x2000],
        palettes: Dmg,
    ) -> (Option<u8>, u8, Irq) {
        self.write_ctrl(lcd_control, &mut irq);
        let clocks = self.clocks + time;

        let mut drawn_ly = None;

        let (clocks, mode) = match (self.mode, clocks) {
            (Mode::OamScan, 80..) => (clocks - 80, Mode::Drawing),
            (Mode::Drawing, 172..) => {
                drawn_ly = self.draw(ly, lcd_control, scx, scy, vram, palettes);

                if self.lcd_status.contains(LcdStatus::HBLANK_INT) {
                    irq.request |= Ints::LCD
                }

                (clocks - 172, Mode::HBlank)
            }
            (Mode::HBlank, 204..) => {
                ly += 1;

                // ly becomes 144 before vblank interrupt
                if ly > 143 {
                    irq.request |= Ints::VBLANK;
                    if self.lcd_status.contains(LcdStatus::VBLANK_INT) {
                        irq.request |= Ints::LCD
                    }

                    (clocks - 204, Mode::VBlank)
                } else {
                    if self.lcd_status.contains(LcdStatus::OAM_INT) {
                        irq.request |= Ints::LCD
                    }

                    (clocks - 204, Mode::OamScan)
                }
            }
            (Mode::VBlank, 456..) => {
                ly += 1;

                if ly > 153 {
                    ly = 0;

                    if self.lcd_status.contains(LcdStatus::OAM_INT) {
                        irq.request |= Ints::LCD;
                    }

                    (clocks - 456, Mode::OamScan)
                } else {
                    (clocks - 456, Mode::VBlank)
                }
            }
            (Mode::None, _) => (0, Mode::None),
            (mode, clock) => (clock, mode),
        };

        if self.lcd_status.contains(LcdStatus::LYC_INT) && self.lyc == ly {
            irq.request |= Ints::LCD;
        }

        self.clocks = clocks;
        self.mode = mode;

        (drawn_ly, ly, irq)
    }

    #[inline(never)]
    fn draw(
        &mut self,
        ly: u8,
        lcd_control: LcdControl,
        scx: u8,
        scy: u8,
        vram: &[u8; 0x2000],
        palettes: Dmg,
    ) -> Option<u8> {
        if ly >= VRAM_HEIGHT {
            return None;
        }

        self.draw_line.fill(Default::default());

        let mut bgbuf = [0u8; VRAM_WIDTH as usize];

        if lcd_control.contains(LcdControl::BG_AND_WINDOW_ENABLE) {
            self.when_bg_and_window_enable(&mut bgbuf, scx, scy, ly, lcd_control, vram, palettes);
            // https://gbdev.io/pandocs/LCDC.html#non-cgb-mode-dmg-sgb-and-cgb-in-compatibility-mode-bg-and-window-display
            // When Bit 0 [LcdControl::BG_AND_WINDOW_ENABLE] is cleared, both background and window become blank (white),
            // and the Window Display Bit [LcdControl::WINDOW_ENABLE] is ignored in that case.
            // Only objects may still be displayed (if enabled in Bit 1).
            if lcd_control.contains(LcdControl::WINDOW_ENABLE) {
                self.when_window_enable(ly, lcd_control, vram, palettes);
            }
        }

        if lcd_control.contains(LcdControl::OBJ_ENABLE) {
            self.when_obj_enable(&bgbuf, ly, lcd_control, vram, palettes);
        }

        Some(ly)
    }

    fn when_bg_and_window_enable(
        &mut self,
        bgbuf: &mut [u8; 160],
        scx: u8,
        scy: u8,
        ly: u8,
        lcd_control: LcdControl,
        vram: &[u8; 0x2000],
        palettes: Dmg,
    ) {
        palettes.get_scanline_after_offset(
            scx,
            ly.wrapping_add(scy),
            vram,
            lcd_control.get_bg_and_window_tile_area(),
            lcd_control.get_bgmap(),
            &mut self.draw_line,
            Some(bgbuf),
        );
    }

    fn when_window_enable(
        &mut self,
        ly: u8,
        lcd_control: LcdControl,
        vram: &[u8; 0x2000],
        palettes: Dmg,
    ) {
        if ly >= self.wy {
            palettes.get_scanline_after_offset(
                self.wx.saturating_sub(7),
                ly - self.wy,
                vram,
                lcd_control.get_bg_and_window_tile_area(),
                lcd_control.get_winmap(),
                &mut self.draw_line,
                None,
            );
        }
    }

    fn when_obj_enable(
        &mut self,
        bgbuf: &[u8; 160],
        ly: u8,
        lcd_control: LcdControl,
        vram: &[u8; 0x2000],
        palettes: Dmg,
    ) {
        for oam in self.oam.chunks(4) {
            let ypos = oam[0];

            if ly + 16 < ypos {
                // This sprite doesn't hit the current ly
                continue;
            }

            let tyoff = ly + 16 - ypos; // ly - (ypos - 16)

            if tyoff >= lcd_control.get_spsize() {
                // This sprite doesn't hit the current ly
                continue;
            }

            let attr = palettes.get_sp_attr(oam[3]);

            let tyoff = if attr.yflip {
                lcd_control.get_spsize() - 1 - tyoff
            } else {
                tyoff
            };

            let ti = oam[2];

            let ti = if lcd_control.get_spsize() == 16 {
                if tyoff >= 8 { ti | 1 } else { ti & 0xfe }
            } else {
                ti
            };
            let tyoff = tyoff % 8;

            let tiles = 0x8000;

            let xpos = oam[1];

            if xpos == 0 || xpos >= VRAM_WIDTH + 8 {
                // the object is off-screen
                // https://gbdev.io/pandocs/OAM.html#byte-1--x-position
                continue;
            }

            let tbase = tiles + u16::from(ti) * 16;
            let mut line = get_tile_line(tbase, tyoff, vram);

            if attr.xflip {
                // we have to shift only if the sprite is partially off-screen (screen left side)
                let offset = 8u8.saturating_sub(xpos);
                line[0] >>= offset;
                line[1] >>= offset;
                // we don't need to call .rev() because we want to keep the "natural" flip of the right to left read
                for x in xpos.saturating_sub(8)..VRAM_WIDTH.min(xpos) {
                    let coli = (line[0] & 1) | ((line[1] & 1) << 1);
                    line[0] >>= 1;
                    line[1] >>= 1;

                    if coli == 0 {
                        // Color index 0 means transparent
                        continue;
                    }

                    let col = attr.palette[usize::from(coli)];

                    let bgcoli = bgbuf[usize::from(x)];

                    if attr.priority && bgcoli != 0 {
                        // If priority is lower than bg color 1-3, don't draw
                        continue;
                    }

                    self.draw_line[usize::from(x)] = col;
                }
            } else {
                // we have to shift only if the sprite is partially off-screen (screen right side)
                let offset = xpos.saturating_sub(VRAM_WIDTH);
                line[0] >>= offset;
                line[1] >>= offset;
                for x in (xpos.saturating_sub(8)..VRAM_WIDTH.min(xpos)).rev() {
                    let coli = (line[0] & 1) | ((line[1] & 1) << 1);
                    line[0] >>= 1;
                    line[1] >>= 1;

                    if coli == 0 {
                        // Color index 0 means transparent
                        continue;
                    }

                    let col = attr.palette[usize::from(coli)];

                    let bgcoli = bgbuf[usize::from(x)];

                    if attr.priority && bgcoli != 0 {
                        // If priority is lower than bg color 1-3, don't draw
                        continue;
                    }

                    self.draw_line[usize::from(x)] = col;
                }
            }
        }
    }
}

fn get_tile_base(tiles: u16, mapbase: u16, tile: Point, vram_bank0: &[u8; 0x2000]) -> u16 {
    let ti = u16::from(tile.x) + u16::from(tile.y) * 32;
    let num = read_vram_bank(mapbase + ti, vram_bank0);

    if tiles == 0x8000 {
        tiles + u16::from(num) * 16
    } else {
        tiles + (0x800 + i16::from(num as i8) * 16) as u16
    }
}

/// https://gbdev.io/pandocs/Tile_Data.html#vram-tile-data
///
/// Each tile occupies 16 bytes, where each line is represented by 2 bytes
fn get_tile_line(tilebase: u16, y_offset: u8, bank: &[u8; 0x2000]) -> [u8; 2] {
    let off = usize::from(tilebase + u16::from(y_offset) * 2 - 0x8000);
    bank[off..=off + 1].try_into().unwrap()
}

fn read_vram_bank(addr: u16, bank: &[u8; 0x2000]) -> u8 {
    let off = addr - 0x8000;
    bank[usize::from(off)]
}
