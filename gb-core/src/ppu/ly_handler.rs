use crate::{StateMachine, ic::Ints, ppu::LcdControl, state::LcdStatus};

const LINE_DURATION_M_CYCLE: u8 = 114;

#[derive(Clone, Default)]
pub struct LyHandler {
    // ly increment logic
    logical_ly: u8,
    clock_count_in_line: u8,
    // lyc logic (needs logical_ly to work)
    delayed_ly: u8,
    already_checked: bool,
}

impl LyHandler {
    // weird behavior of dmg described by the cycle accurate gameboy docs
    fn is_ly_check_disabled(&self) -> bool {
        self.logical_ly > 0 && self.clock_count_in_line == 0
            || self.logical_ly == 153 && self.clock_count_in_line == 2
    }
}

impl StateMachine for LyHandler {
    fn execute(&mut self, state: &mut crate::state::State, _: u64) {
        if !state.lcd_control.contains(LcdControl::LCD_PPU_ENABLE) {
            return;
        }
        // ly increment logic
        match self.clock_count_in_line {
            LINE_DURATION_M_CYCLE => {
                self.clock_count_in_line = 0;
                if self.logical_ly != 153 {
                    // the increment is handled differently on line 153
                    state.ly += 1;
                }
                self.logical_ly = (self.logical_ly + 1) % 154;
            }
            1 if state.ly == 153 => state.ly = 0,
            _ => {}
        }

        // lyc logic
        if state.lyc == self.delayed_ly && !self.is_ly_check_disabled() {
            if !self.already_checked && state.lcd_status.contains(LcdStatus::LYC_INT) {
                state.interrupt_flag.insert(Ints::LCD);
                self.already_checked = true
            }
            state.lcd_status.insert(LcdStatus::LYC_EQUAL_TO_LY);
        } else {
            state.lcd_status.remove(LcdStatus::LYC_EQUAL_TO_LY);
        }

        if state.ly != self.delayed_ly {
            self.delayed_ly = state.ly;
            self.already_checked = false;
        }

        self.clock_count_in_line += 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::{StateMachine, ppu::ly_handler::LyHandler, state::State};

    #[test]
    fn ly_incrementer() {
        let mut ly_incrementer = LyHandler::default();
        let mut state = State::new(&[]);
        // cycle 0 (line 0)
        ly_incrementer.execute(&mut state, 0);
        assert_eq!(0, state.ly);
        // cycle 113 (line 0)
        for _ in 0..113 {
            ly_incrementer.execute(&mut state, 0);
        }
        assert_eq!(0, state.ly);
        // cycle 0 (line 1)
        ly_incrementer.execute(&mut state, 0);
        assert_eq!(1, state.ly);
        // cycle 113 (line 152)
        for _ in 0..(114 * 152 - 1) {
            ly_incrementer.execute(&mut state, 0);
        }
        assert_eq!(152, state.ly);
        // cycle 0 (line 153)
        ly_incrementer.execute(&mut state, 0);
        assert_eq!(153, state.ly);
        // cycle 1 (line 153)
        ly_incrementer.execute(&mut state, 0);
        assert_eq!(0, state.ly);
        // cycle 113 (line 153)
        for _ in 0..112 {
            ly_incrementer.execute(&mut state, 0);
        }
        assert_eq!(0, state.ly);
        // cycle 0 (line 0)
        ly_incrementer.execute(&mut state, 0);
        assert_eq!(0, state.ly);
        // cycle 113 (line 0)
        for _ in 0..113 {
            ly_incrementer.execute(&mut state, 0);
        }
        assert_eq!(0, state.ly);
        // cycle 0 (line 1)
        ly_incrementer.execute(&mut state, 0);
        assert_eq!(1, state.ly);
    }
}
