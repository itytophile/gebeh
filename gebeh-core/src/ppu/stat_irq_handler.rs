use crate::{
    ppu::Ppu,
    state::{LcdStatus, State},
};

pub struct StatIrqHandler(bool);

impl StatIrqHandler {
    // pub fn execute(&mut self, ppu: &Ppu, state: &State) {
    //     // don't check the ppu mode via state. There are timing issues with it.
    //     let lol = match ppu {
    //         Ppu::OamScan { dots_count: 5.., .. } => state.lcd_status.contains(LcdStatus::OAM_INT),
    //         Ppu::HorizontalBlank { dots_count: 8.., .. } => state.lcd_status.contains(LcdStatus::HBLANK_INT),
    //         Ppu::VerticalBlankScanline { remaining_dots } | Ppu::OamScan { dots_count: ..5, .. } => state.lcd_status.contains(LcdStatus::OAM_INT) | state.lcd_status.contains(LcdStatus::VBLANK_INT),
    //         _ => false
    //     };
    // }
}
