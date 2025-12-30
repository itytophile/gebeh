use crate::{
    ppu::Ppu,
    state::{Interruptions, LcdStatus, State},
};

#[derive(Default, Clone)]
pub struct StatIrqHandler(bool);

impl StatIrqHandler {
    pub fn execute(&mut self, ppu: &Ppu, state: &mut State) {
        // don't check the ppu mode via state. There are timing issues with it.
        let stat_mode_irq = match ppu {
            Ppu::OamScan {
                dots_count: 5.., ..
            }
            | Ppu::Drawing {
                dots_count: ..5, ..
            } => state.lcd_status.contains(LcdStatus::OAM_INT),
            Ppu::HorizontalBlank {
                dots_count: 8.., ..
            }
            | Ppu::VerticalBlankScanline { dots_count: ..5 } => {
                state.lcd_status.contains(LcdStatus::HBLANK_INT)
            }
            Ppu::VerticalBlankScanline { dots_count: 5.. }
            | Ppu::OamScan {
                dots_count: ..5, ..
            } => {
                // according to https://github.com/Gekkio/mooneye-test-suite/blob/443f6e1f2a8d83ad9da051cbb960311c5aaaea66/acceptance/ppu/vblank_stat_intr-GS.s
                state.lcd_status.contains(LcdStatus::OAM_INT)
                    | state.lcd_status.contains(LcdStatus::VBLANK_INT)
            }
            _ => false,
        };

        let stat_irq = stat_mode_irq
            || (state.lcd_status.contains(LcdStatus::LYC_INT) && state.ly == state.lyc);

        if stat_irq == self.0 {
            return;
        }

        self.0 = stat_irq;

        // rising edge described by https://raw.githubusercontent.com/geaz/emu-gameboy/master/docs/The%20Cycle-Accurate%20Game%20Boy%20Docs.pdf
        if self.0 {
            state.interrupt_flag.insert(Interruptions::LCD);
        }
    }
}
