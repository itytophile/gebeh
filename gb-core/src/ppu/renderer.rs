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
        Color, LcdControl, ObjectAttribute,
        background_fetcher::{BackgroundFetcher, BackgroundFetcherStep},
        fifos::Fifos,
        sprite_fetcher::SpriteFetcher,
    },
    state::{Scrolling, State},
};

#[derive(Clone)]
pub struct Renderer {
    background_pixel_fetcher: BackgroundFetcher,
    sprite_pixel_fetcher: SpriteFetcher,
    rendering_state: RenderingState,
    pub objects: ArrayVec<ObjectAttribute, 10>,
    pub scanline: ArrayVec<Color, 160>,
    first_pixels_to_skip: u8,
    wx_condition: bool,
}

impl Renderer {
    pub fn new(objects: ArrayVec<ObjectAttribute, 10>, scx_at_scanline_start: u8) -> Self {
        log::warn!(
            "Will render with {} objects and initial scrolling of {}",
            objects.len(),
            scx_at_scanline_start
        );
        if let Some(obj) = objects.last() {
            log::warn!("First object at {}", obj.x);
        }
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

    pub fn execute(&mut self, state: &State, dots_count: u16, window_y: &mut Option<u8>) {
        let cursor = i16::from(self.rendering_state.fifos.get_shifted_count())
            - i16::from(self.first_pixels_to_skip);

        // yes can be triggered multiple times if wx changes during the same scanline
        if state.lcd_control.contains(LcdControl::WINDOW_ENABLE)
            && cursor == i16::from(state.wx + 1)
            && let Some(window_y) = window_y
            && !self.wx_condition
        {
            self.background_pixel_fetcher = BackgroundFetcher {
                step: BackgroundFetcherStep::WaitingForScrollRegisters,
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
            && state.lcd_control.contains(LcdControl::WINDOW_ENABLE)
        {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &state.video_ram,
                state.lcd_control.get_window_tile_map_address(),
                Scrolling::default(),
                // - 1 because we increment it at window initialization
                *window_y - 1,
                !state.lcd_control.contains(LcdControl::BG_AND_WINDOW_TILES),
            );
        } else {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &state.video_ram,
                state.lcd_control.get_bg_tile_map_address(),
                state.get_scrolling(),
                state.ly,
                !state.lcd_control.contains(LcdControl::BG_AND_WINDOW_TILES),
            );
        }

        self.sprite_pixel_fetcher.execute(
            cursor,
            &mut self.rendering_state,
            &mut self.objects,
            state,
            dots_count,
        );

        if self.rendering_state.fifos.is_background_empty() || !self.rendering_state.is_shifting {
            return;
        }

        if cursor >= 8 {
            log::warn!("{dots_count}: pushing to lcd");
            self.scanline.push(self.rendering_state.fifos.render_pixel(
                state.bgp_register,
                state.obp0,
                state.obp1,
                state.lcd_control.contains(LcdControl::BG_AND_WINDOW_ENABLE),
            ));
        }

        log::warn!("{dots_count}: shifting");
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
    use crate::{
        WIDTH,
        ppu::{LcdControl, renderer::Renderer},
        state::State,
    };

    // all timings are +2 compared to pandocs timings
    const MINIMUM_TIME: u16 = 174;

    fn get_timing(state: &State, mut window_y: Option<u8>) -> u16 {
        let mut renderer = Renderer::new(Default::default(), 0);
        let mut dots = 0;
        while renderer.scanline.len() < usize::from(WIDTH) {
            renderer.execute(state, dots, &mut window_y);
            dots += 1;
        }
        dots
    }

    #[test]
    fn normal_timing() {
        assert_eq!(get_timing(&State::new(&[]), None), MINIMUM_TIME);
    }

    #[test]
    fn with_window() {
        let mut state = State::new(&[]);
        state.lcd_control.insert(LcdControl::WINDOW_ENABLE);

        for wx in 0..167 {
            state.wx = wx;
            // https://gbdev.io/pandocs/Rendering.html#mode-3-length
            // Citation: After the last non-window pixel is emitted, a 6-dot penalty is incurred
            assert_eq!(
                get_timing(&state, Some(0)),
                MINIMUM_TIME + 6,
                "Bad timing with WX = {wx}"
            );
        }

        // the window is not visible
        state.wx = 167;
        assert_eq!(
            get_timing(&state, Some(0)),
            MINIMUM_TIME,
            "Bad timing with WX = 167"
        );
    }
}
