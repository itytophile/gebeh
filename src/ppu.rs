use crate::{
    StateMachine,
    gpu::{Dmg, Gpu, to_palette},
    ic::{Ints, Irq},
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
        let interrupt_enable = state.interrupt_enable;
        let interrupt_flag = state.interrupt_flag;
        let ly = state.ly;
        let lcd_control = state.lcd_control;
        let scx = state.scx;
        let scy = state.scy;
        let bgp = state.bgp_register;
        let obp0 = state.obp0;
        let obp1 = state.obp1;

        // TODO revoir comment ça gère les interruptions ici
        let (drawn_ly, ly, irq) = self.gpu.step(
            4,
            Irq {
                enable: interrupt_enable,
                request: interrupt_flag,
            },
            ly,
            lcd_control,
            scx,
            scy,
            &state.video_ram,
            Dmg {
                bg_palette: to_palette(bgp),
                obj_palette0: to_palette(obp0),
                obj_palette1: to_palette(obp1),
            },
            state.wy,
            state.wx,
        );

        self.drawn_ly = drawn_ly;

        move |mut state| {
            state.set_ly(ly);
            for flag in [Ints::VBLANK, Ints::LCD] {
                if interrupt_flag.contains(flag) && !irq.request.contains(flag) {
                    state.get_if_mut().remove(flag);
                }
                if !interrupt_flag.contains(flag) && irq.request.contains(flag) {
                    state.get_if_mut().insert(flag);
                }
                if interrupt_enable.contains(flag) && !irq.enable.contains(flag) {
                    state.get_ie_mut().remove(flag);
                }
                if !interrupt_enable.contains(flag) && irq.enable.contains(flag) {
                    state.get_ie_mut().insert(flag);
                }
            }
        }
    }
}
