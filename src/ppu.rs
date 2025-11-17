use crate::{
    StateMachine,
    gpu::Gpu,
    ic::Irq,
    state::{State, WriteOnlyState},
};

#[derive(Default)]
pub struct Ppu {
    pub gpu: Gpu,
    pub drawn_ly: Option<u8>,
}

// 4 dots per Normal Speed M-cycle
// One frame: 70224 dots
// A frame consists of 154 scanlines
// 456 dots per scanline

impl StateMachine for Ppu {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        let ie = state.interrupt_enable;
        let ifl = state.interrupt_flag;
        let ly = state.ly;
        let lcd_control = state.lcd_control;
        let scx = state.scx;
        let scy = state.scy;
        let vram = state.video_ram;

        move |state| {
            self.drawn_ly = self.gpu.step(
                4,
                Irq {
                    enable: ie,
                    request: ifl,
                },
                ly,
                lcd_control,
                scx,
                scy,
                state,
                &vram
            );
        }
    }
}
