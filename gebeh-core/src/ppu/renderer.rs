// relevant docs
// https://github.com/Ashiepaws/GBEDG/blob/97f198d330a51be558aa8fc9f3f0760846d02d95/ppu/index.md#background-pixel-fetching
// https://gbdev.io/pandocs/pixel_fifo.html#fifo-pixel-fetcher
// http://blog.kevtris.org/blogfiles/Nitty%20Gritty%20Gameboy%20VRAM%20Timing.txt
// https://www.reddit.com/r/EmuDev/comments/s6cpis/gameboy_trying_to_understand_sprite_fifo_behavior/ <- spitting facts
//
// The ppu can't do two tile fetches at the same time, so if we fetch a sprite for an object
// then we must pause the background/window tile fetch.
// A sprite fetch is triggered only if the background fifo has pixels.
// according to "Gameboy Emulator Development Guide", the background pixel fetcher is not only paused, but reset.
// During the object sprite fetch, both the LCD AND the background FIFO are paused.
// The Sprite FIFO has not the same behavior as the Background FIFO. The background pixel fetcher always wait for
// the background fifo to be empty before refilling it. However the sprite pixel fetcher, is only replacing the "empty slots"
// of the Sprite FIFO, to keep the pixels of the previous sprite.
// We know from pandocs (https://gbdev.io/pandocs/OAM.html#drawing-priority) that if two sprites overlap, opaque colors are drawn over
// the transparent ones (yes) so I assume the sprite pixel fetcher refills the sprite FIFO with an OR operation.
// But what about the priority flag ? (https://gbdev.io/pandocs/OAM.html#byte-3--attributesflags) we will keep a fifo for that
// and try to guess along the way.

use arrayvec::ArrayVec;

use crate::{
    ppu::{
        LcdControl, ObjectAttribute, PpuState, background_fetcher::BackgroundFetcher, fifos::Fifos,
        scanline::ScanlineBuilder, sprite_fetcher::SpriteFetcher,
    },
    state::{Scrolling, State},
};

#[derive(Clone)]
pub struct Renderer {
    background_pixel_fetcher: BackgroundFetcher,
    sprite_pixel_fetcher: SpriteFetcher,
    rendering_state: RenderingState,
    pub objects: ArrayVec<ObjectAttribute, 10>,
    pub scanline: ScanlineBuilder,
    pub first_pixels_to_skip: u8,
    wx_condition: bool,
}

impl Renderer {
    pub fn new(objects: ArrayVec<ObjectAttribute, 10>, scx_at_scanline_start: u8) -> Self {
        Self {
            background_pixel_fetcher: Default::default(),
            rendering_state: RenderingState {
                is_shifting: true,
                is_sprite_fetching_enable: false,
                fifos: Default::default(),
            },
            sprite_pixel_fetcher: Default::default(),
            scanline: Default::default(),
            objects,
            first_pixels_to_skip: scx_at_scanline_start % 8,
            wx_condition: false,
        }
    }

    pub fn execute(
        &mut self,
        state: &State,
        dots_count: u16,
        window_y: &mut Option<u8>,
        ppu_state: &PpuState,
        cycles: u64,
    ) {
        let cursor = i16::from(self.rendering_state.fifos.get_shifted_count())
            - i16::from(self.first_pixels_to_skip);

        // yes can be triggered multiple times if wx changes during the same scanline
        if ppu_state.lcd_control.contains(LcdControl::WINDOW_ENABLE)
            && cursor == i16::from(state.wx + 1)
            && let Some(window_y) = window_y
            && !self.wx_condition
        {
            self.background_pixel_fetcher = BackgroundFetcher {
                step: Default::default(),
                x: 1,
            };
            self.rendering_state.fifos.reset_background();
            self.wx_condition = true;
            *window_y += 1;
        }

        // those systems can run "concurrently"

        // will hopefully reproduce the glitch described by https://gbdev.io/pandocs/Scrolling.html#window
        if let Some(window_y) = window_y
            && self.wx_condition
            && ppu_state.lcd_control.contains(LcdControl::WINDOW_ENABLE)
        {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &state.video_ram,
                ppu_state.lcd_control.get_window_tile_map_address(),
                Scrolling::default(),
                // - 1 because we increment it at window initialization
                *window_y - 1,
                !ppu_state
                    .lcd_control
                    .contains(LcdControl::BG_AND_WINDOW_TILES),
            );
        } else {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &state.video_ram,
                ppu_state.lcd_control.get_bg_tile_map_address(),
                state.get_scrolling(),
                ppu_state.ly,
                !ppu_state
                    .lcd_control
                    .contains(LcdControl::BG_AND_WINDOW_TILES),
            );
        }

        self.sprite_pixel_fetcher.execute(
            cursor,
            &mut self.rendering_state,
            &mut self.objects,
            state,
            ppu_state,
            dots_count,
        );

        if self.rendering_state.fifos.is_background_empty() || !self.rendering_state.is_shifting {
            return;
        }

        if cursor >= 8 {
            // log::info!(
            //     "{cycles} pushing pixel on {} with bgp 0b{:08b}",
            //     self.scanline.len(),
            //     state.bgp_register
            // );
            self.scanline.push_pixel(
                self.rendering_state.fifos.render_pixel(
                    state.bgp_register,
                    state.obp0,
                    state.obp1,
                    ppu_state
                        .lcd_control
                        .contains(LcdControl::BG_AND_WINDOW_ENABLE),
                ),
            );
        }

        self.rendering_state.fifos.shift();
    }
}

#[derive(Clone)]
pub struct RenderingState {
    pub is_shifting: bool,
    pub is_sprite_fetching_enable: bool,
    pub fifos: Fifos,
}

#[cfg(test)]
mod tests {
    use arrayvec::ArrayVec;

    use crate::{
        WIDTH,
        ppu::{LcdControl, ObjectAttribute, ObjectFlags, PpuState, renderer::Renderer},
        state::State,
    };

    // all timings are +2 compared to pandocs timings
    const MINIMUM_TIME: u16 = 174;

    fn get_timing(
        state: &State,
        mut window_y: Option<u8>,
        objects: ArrayVec<ObjectAttribute, 10>,
        ppu_state: &PpuState,
    ) -> u16 {
        let mut renderer = Renderer::new(objects, state.scx);
        let mut dots = 0;
        while renderer.scanline.len() < WIDTH {
            renderer.execute(state, dots, &mut window_y, ppu_state, 0);
            dots += 1;
        }
        dots
    }

    #[test]
    fn normal_timing() {
        assert_eq!(
            get_timing(
                &State::default(),
                None,
                Default::default(),
                &Default::default()
            ),
            MINIMUM_TIME
        );
    }

    #[test]
    fn with_window() {
        let mut state = State::default();

        // https://gbdev.io/pandocs/Scrolling.html#ff4aff4b--wy-wx-window-y-position-x-position-plus-7
        // Citation: The Window is visible (if enabled) when both coordinates are in the ranges WX=0..166, WY=0..143 respectively
        for wx in 0..167 {
            state.wx = wx;
            // https://gbdev.io/pandocs/Rendering.html#mode-3-length
            // Citation: After the last non-window pixel is emitted, a 6-dot penalty is incurred
            assert_eq!(
                get_timing(
                    &state,
                    Some(0),
                    Default::default(),
                    &PpuState {
                        lcd_control: LcdControl::WINDOW_ENABLE,
                        ly: 0
                    }
                ),
                MINIMUM_TIME + 6,
                "Bad timing with WX = {wx}"
            );
        }

        // the window is not visible
        state.wx = 167;
        assert_eq!(
            get_timing(
                &state,
                Some(0),
                Default::default(),
                &PpuState {
                    lcd_control: LcdControl::WINDOW_ENABLE,
                    ly: 0
                }
            ),
            MINIMUM_TIME,
            "Bad timing with WX = 167"
        );
    }

    #[test]
    fn with_objects() {
        let state = State::default();
        let objects = ArrayVec::from_iter([ObjectAttribute {
            flags: ObjectFlags::empty(),
            tile_index: 0,
            x: 0,
            y: 0,
        }]);
        // https://gbdev.io/pandocs/Rendering.html#obj-penalty-algorithm
        // Citation: an OBJ with an OAM X position of 0 always incurs a 11-dot penalty
        assert_eq!(
            get_timing(
                &state,
                None,
                objects,
                &PpuState {
                    lcd_control: LcdControl::OBJ_ENABLE,
                    ly: 0
                }
            ),
            MINIMUM_TIME + 11
        );
    }

    #[test]
    fn with_scroll_x() {
        let mut state = State::default();

        for scx in 0..=u8::MAX {
            state.scx = scx;
            // https://gbdev.io/pandocs/Rendering.html#mode-3-length
            // Citation: At the very beginning of Mode 3, rendering is paused for SCX % 8 dots
            assert_eq!(
                get_timing(&state, None, Default::default(), &Default::default()),
                MINIMUM_TIME + u16::from(scx % 8),
                "Failed with scx {scx}"
            );
        }
    }
}
