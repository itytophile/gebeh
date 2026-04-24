// toute la logique de lecture/ecriture peut être extériorisé. Peut-être que c'est élément "bus" qui s'en occupera.

use crate::ppu::LcdControl;

fn wy_match(lcd_control: LcdControl, wy: u8, ly: u8) -> bool {
    lcd_control.contains(LcdControl::WINDOW_ENABLE) && wy == ly
}

fn wxy_match(wy_latch: bool, bg_win_counter: u8, wx: u8) -> bool {
    wy_latch && bg_win_counter == wx
}

struct WyLatch {
    match_ff: bool,
    latch: bool,
}

impl WyLatch {
    fn execute(&mut self, wy_match: bool, hclk: bool, ppu_reset: bool, is_mode1: bool) -> bool {
        if ppu_reset {
            self.match_ff = false;
            self.latch = false;
            return false;
        }

        let old_match_ff = self.match_ff;

        if hclk {
            self.match_ff = wy_match;
        }

        if is_mode1 {
            self.latch = false;
            return false;
        }

        self.latch |= !old_match_ff && self.match_ff;

        self.latch
    }
}

struct WyRegister(u8);

impl WyRegister {
    fn execute(&mut self) {}
}
