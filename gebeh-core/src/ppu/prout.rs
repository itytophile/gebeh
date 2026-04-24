// toute la logique de lecture/ecriture peut être extériorisé. Peut-être que c'est élément "bus" qui s'en occupera.

use crate::ppu::LcdControl;

fn wy_match(lcd_control: LcdControl, wy: u8, ly: u8) -> WyMatch {
    WyMatch(lcd_control.contains(LcdControl::WINDOW_ENABLE) && wy == ly)
}

fn wxy_match(wy_latch: WyLatch, bg_win_counter: u8, wx: u8) -> WxyMatch {
    WxyMatch(wy_latch.0 && bg_win_counter == wx)
}

struct WxRst(bool);

fn wx_rst(atej: bool, lcd_control: LcdControl, ppu_reset: bool) -> WxRst {
    WxRst(atej || ppu_reset || !lcd_control.contains(LcdControl::WINDOW_ENABLE))
}

struct SyncedInWindowState {
    nopa: FlipFlop,
}

struct SyncedInWindow(bool);

impl SyncedInWindowState {
    fn update(&mut self, in_window: InWindow, ppu_4mhz: bool) -> SyncedInWindow {
        SyncedInWindow(self.nopa.update(!ppu_4mhz, in_window.0) && in_window.0)
    }
}

struct WyLatchState {
    match_ff: FlipFlop,
    latch: Latch,
}

struct WyMatch(bool);

struct WyLatch(bool);

struct WxyMatch(bool);

// ignore ppu_reset because it resets everything so whatever
impl WyLatchState {
    fn update(&mut self, wy_match: WyMatch, hclk: bool, is_mode1: bool) -> WyLatch {
        let match_ff = self.match_ff.update(hclk, wy_match.0);
        WyLatch(self.latch.update(match_ff, is_mode1))
    }
}

struct InWindowState {
    pyco: FlipFlop,
    nunu: FlipFlop,
    pynu: Latch,
}

struct InWindow(bool);

impl InWindowState {
    fn update(
        &mut self,
        wxy_match: WxyMatch,
        segu: bool,
        ppu_4mhz: bool,
        atej: bool,
        lcd_control: LcdControl,
    ) -> InWindow {
        let pyco = self.pyco.update(!segu, wxy_match.0);
        let nunu = self.nunu.update(ppu_4mhz, pyco);
        InWindow(self.pynu.update(
            nunu,
            atej || !lcd_control.contains(LcdControl::WINDOW_ENABLE),
        ))
    }
}

struct FlipFlop(bool);

impl FlipFlop {
    fn update(&mut self, clk: bool, d: bool) -> bool {
        if clk {
            self.0 = d;
        }
        self.0
    }
}

struct Latch(bool);

impl Latch {
    fn update(&mut self, set: bool, reset: bool) -> bool {
        if set && reset {
            panic!("Invalid state latch");
        }
        if set {
            self.0 = true;
        } else if reset {
            self.0 = false;
        }
        self.0
    }
}

struct WyRegister(u8);

impl WyRegister {
    fn execute(&mut self) {}
}
