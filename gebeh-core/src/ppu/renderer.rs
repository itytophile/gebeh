// relevant docs
// https://github.com/Ashiepaws/GBEDG/blob/97f198d330a51be558aa8fc9f3f0760846d02d95/ppu/index.md#background-pixel-fetching
// https://gbdev.io/pandocs/pixel_fifo.html#fifo-pixel-fetcher
// http://blog.kevtris.org/blogfiles/Nitty%20Gritty%20Gameboy%20VRAM%20Timing.txt
// https://www.reddit.com/r/EmuDev/comments/s6cpis/gameboy_trying_to_understand_sprite_fifo_behavior/ <- spitting facts

use arrayvec::ArrayVec;

use crate::{
    Ram,
    ppu::{
        LcdControl, PpuState, Scrolling, Sprite,
        background_fetcher::{
            BackgroundFetcher, BackgroundFetcherStep, CgbBackgroundFetcher,
            CgbBackgroundFetcherStep,
        },
        color_palettes::ColorPalettes,
        fifos::{CgbFifos, DmgFifos},
        scanline::{DmgScanlineBuilder, ScanlineBuilder},
        sprite_fetcher::{CgbSpriteFetcher, SpriteFetcher},
        vram::{CgbVram, DmgVram},
    },
};

#[derive(Clone)]
pub enum RendererStep {
    DummyFetch,
    AfterDummy {
        first_pixels_to_skip: u8,
        saved_wx: Option<u8>,
    },
}

pub trait Renderer: Clone {
    type Vram: Ram;
    type Extra: Default + Clone;
    type ScanlineBuilder: ScanlineBuilder;
    fn new(objects: ArrayVec<Sprite, 10>) -> Self;
    fn execute(
        &mut self,
        window_y: &mut Option<u8>,
        ppu_state: &PpuState<Self::Vram>,
        extra: &Self::Extra,
        ly: u8,
        cycle: u64,
    );
    fn get_scanline_builder(&self) -> &Self::ScanlineBuilder;
}

#[derive(Clone)]
pub struct DmgRenderer {
    background_pixel_fetcher: BackgroundFetcher,
    sprite_pixel_fetcher: SpriteFetcher,
    rendering_state: RenderingState,
    fifos: DmgFifos,
    objects: ArrayVec<Sprite, 10>,
    pub scanline: DmgScanlineBuilder,
    step: RendererStep,
}

impl DmgRenderer {
    pub fn new(objects: ArrayVec<Sprite, 10>) -> Self {
        Self {
            background_pixel_fetcher: Default::default(),
            rendering_state: RenderingState {
                is_shifting: true,
                is_sprite_fetching_enable: false,
            },
            fifos: Default::default(),

            sprite_pixel_fetcher: Default::default(),
            scanline: Default::default(),
            objects,
            step: RendererStep::DummyFetch,
        }
    }

    pub(super) fn execute(
        &mut self,
        window_y: &mut Option<u8>,
        ppu_state: &PpuState,
        ly: u8,
        _: u64,
    ) {
        let RendererStep::AfterDummy {
            first_pixels_to_skip,
            ref mut saved_wx,
        } = self.step
        else {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &mut self.fifos,
                ppu_state.video_ram.get_inner(),
                ppu_state.get_bg_tile_map_address(),
                ppu_state.get_scrolling(),
                ly,
                ppu_state.is_signed_addressing(),
            );

            if let BackgroundFetcherStep::Ready(_) = self.background_pixel_fetcher.step {
                self.step = RendererStep::AfterDummy {
                    // https://gbdev.io/pandocs/Scrolling.html#scrolling
                    // Citation: The scroll registers are re-read on each tile fetch, except for
                    // the low 3 bits of SCX, which are only read at the beginning of the scanline
                    //
                    // And according to mealybug, it's read after the dummy fetch
                    first_pixels_to_skip: ppu_state.scx % 8,
                    saved_wx: None,
                };
            }

            return;
        };
        let cursor = i16::from(self.fifos.get_shifted_count()) - i16::from(first_pixels_to_skip);

        // yes can be triggered multiple times if wx changes during the same scanline
        if ppu_state
            .old_lcd_control
            .contains(LcdControl::WINDOW_ENABLE)
            && (cursor == i16::from(ppu_state.old_old_wx.saturating_add(1))
                // strange race condition showed by mealybug and my Game Boy Pocket
                || (cursor == i16::from(ppu_state.old_old_wx.saturating_add(2))
                    && !ppu_state
                        .old_old_lcd_control
                        .contains(LcdControl::WINDOW_ENABLE)))
            && let Some(window_y) = window_y
            && Some(ppu_state.old_old_wx) != *saved_wx
        {
            if saved_wx.is_none() {
                self.background_pixel_fetcher = BackgroundFetcher {
                    step: Default::default(),
                    x: 1,
                };
                self.fifos.reset_background();
                *window_y = window_y.wrapping_add(1);
            } else if self.fifos.is_background_empty() {
                // according to mealybug m3_wx_4_change
                self.fifos.insert_window_reactivation_pixel();
            }

            *saved_wx = Some(ppu_state.old_old_wx);

            // according to mealybug "due to window activating one T-cycle later when WX = 0 and SCX > 0"
            if ppu_state.old_old_wx == 0 && first_pixels_to_skip > 0 {
                return;
            }
        }

        // those systems can run "concurrently"

        if let Some(window_y) = window_y
            && saved_wx.is_some()
        {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &mut self.fifos,
                ppu_state.video_ram.get_inner(),
                ppu_state.get_window_tile_map_address(),
                Scrolling::default(),
                // - 1 because we increment it at window initialization
                window_y.wrapping_sub(1),
                ppu_state.is_signed_addressing(),
            );
            // according to mealybug, when the window is disabled, we have to wait for the fetch to end
            // before disabling the window for real
            if matches!(
                self.background_pixel_fetcher.step,
                // yeah it works with this step, don't know why
                BackgroundFetcherStep::FetchingTileIndex { .. }
            ) && !ppu_state.lcd_control.contains(LcdControl::WINDOW_ENABLE)
            {
                *saved_wx = None;
            }
        } else {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &mut self.fifos,
                ppu_state.video_ram.get_inner(),
                ppu_state.get_bg_tile_map_address(),
                ppu_state.get_scrolling(),
                ly,
                ppu_state.is_signed_addressing(),
            );
        }

        self.sprite_pixel_fetcher.execute(
            cursor,
            &mut self.rendering_state,
            &mut self.fifos,
            &mut self.objects,
            ppu_state.lcd_control,
            ppu_state.video_ram.get_inner(),
            ly,
        );

        if self.fifos.is_background_empty() || !self.rendering_state.is_shifting {
            return;
        }

        if cursor >= 8 {
            self.scanline.push_pixel(self.fifos.render_pixel(
                ppu_state.get_effective_bgp(),
                ppu_state.obp0,
                ppu_state.obp1,
                ppu_state.is_background_enabled(),
                ppu_state.is_obj_enabled(),
            ));
        }

        self.fifos.shift();
    }
}

impl Renderer for DmgRenderer {
    type Vram = DmgVram;
    type Extra = ();
    type ScanlineBuilder = DmgScanlineBuilder;

    fn new(objects: ArrayVec<Sprite, 10>) -> Self {
        Self::new(objects)
    }

    fn execute(
        &mut self,
        window_y: &mut Option<u8>,
        ppu_state: &PpuState<Self::Vram>,
        _: &Self::Extra,
        ly: u8,
        cycle: u64,
    ) {
        self.execute(window_y, ppu_state, ly, cycle);
    }

    fn get_scanline_builder(&self) -> &Self::ScanlineBuilder {
        &self.scanline
    }
}

// yea I'm copy pasting everything. The original rendering logic has specific Game Boy Pocket behavior.
// So there is a lot of chance that in the future, the cgb rendering logic will be modified.

#[derive(Clone)]
pub struct CgbRenderer {
    background_pixel_fetcher: CgbBackgroundFetcher,
    sprite_pixel_fetcher: CgbSpriteFetcher,
    rendering_state: RenderingState,
    fifos: CgbFifos,
    pub objects: ArrayVec<Sprite, 10>,
    pub scanline: ArrayVec<u16, 160>,
    step: RendererStep,
}

impl CgbRenderer {
    pub fn new(objects: ArrayVec<Sprite, 10>) -> Self {
        Self {
            background_pixel_fetcher: Default::default(),
            rendering_state: RenderingState {
                is_shifting: true,
                is_sprite_fetching_enable: false,
            },
            fifos: Default::default(),

            sprite_pixel_fetcher: Default::default(),
            scanline: Default::default(),
            objects,
            step: RendererStep::DummyFetch,
        }
    }

    pub(super) fn execute(
        &mut self,
        window_y: &mut Option<u8>,
        ppu_state: &PpuState<CgbVram>,
        color_palettes: &ColorPalettes,
        ly: u8,
        _: u64,
    ) {
        let RendererStep::AfterDummy {
            first_pixels_to_skip,
            ref mut saved_wx,
        } = self.step
        else {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &mut self.fifos,
                ppu_state.video_ram.get_inner(),
                ppu_state.get_bg_tile_map_address(),
                ppu_state.get_scrolling(),
                ly,
                ppu_state.is_signed_addressing(),
            );

            if let CgbBackgroundFetcherStep::Ready { .. } = self.background_pixel_fetcher.step {
                self.step = RendererStep::AfterDummy {
                    // https://gbdev.io/pandocs/Scrolling.html#scrolling
                    // Citation: The scroll registers are re-read on each tile fetch, except for
                    // the low 3 bits of SCX, which are only read at the beginning of the scanline
                    //
                    // And according to mealybug, it's read after the dummy fetch
                    first_pixels_to_skip: ppu_state.scx % 8,
                    saved_wx: None,
                };
            }

            return;
        };
        let cursor = i16::from(self.fifos.get_shifted_count()) - i16::from(first_pixels_to_skip);

        // yes can be triggered multiple times if wx changes during the same scanline
        if ppu_state
            .old_lcd_control
            .contains(LcdControl::WINDOW_ENABLE)
            && (cursor == i16::from(ppu_state.old_old_wx.saturating_add(1))
                // strange race condition showed by mealybug and my Game Boy Pocket
                || (cursor == i16::from(ppu_state.old_old_wx.saturating_add(2))
                    && !ppu_state
                        .old_old_lcd_control
                        .contains(LcdControl::WINDOW_ENABLE)))
            && let Some(window_y) = window_y
            && Some(ppu_state.old_old_wx) != *saved_wx
        {
            if saved_wx.is_none() {
                self.background_pixel_fetcher = CgbBackgroundFetcher {
                    step: Default::default(),
                    x: 1,
                };
                self.fifos.reset_background();
                *window_y = window_y.wrapping_add(1);
            }

            *saved_wx = Some(ppu_state.old_old_wx);

            // according to mealybug "due to window activating one T-cycle later when WX = 0 and SCX > 0"
            if ppu_state.old_old_wx == 0 && first_pixels_to_skip > 0 {
                return;
            }
        }

        // those systems can run "concurrently"

        if let Some(window_y) = window_y
            && saved_wx.is_some()
        {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &mut self.fifos,
                ppu_state.video_ram.get_inner(),
                ppu_state.get_window_tile_map_address(),
                Scrolling::default(),
                // - 1 because we increment it at window initialization
                window_y.wrapping_sub(1),
                ppu_state.is_signed_addressing(),
            );
            // according to mealybug, when the window is disabled, we have to wait for the fetch to end
            // before disabling the window for real
            if matches!(
                self.background_pixel_fetcher.step,
                // yeah it works with this step, don't know why
                CgbBackgroundFetcherStep::FetchingTileIndex { .. }
            ) && !ppu_state.lcd_control.contains(LcdControl::WINDOW_ENABLE)
            {
                *saved_wx = None;
            }
        } else {
            self.background_pixel_fetcher.execute(
                &mut self.rendering_state,
                &mut self.fifos,
                ppu_state.video_ram.get_inner(),
                ppu_state.get_bg_tile_map_address(),
                ppu_state.get_scrolling(),
                ly,
                ppu_state.is_signed_addressing(),
            );
        }

        self.sprite_pixel_fetcher.execute(
            cursor,
            &mut self.rendering_state,
            &mut self.fifos,
            &mut self.objects,
            ppu_state.lcd_control,
            ppu_state.video_ram.get_inner(),
            ly,
        );

        if self.fifos.is_background_empty() || !self.rendering_state.is_shifting {
            return;
        }

        if cursor >= 8 {
            self.scanline.push(self.fifos.render_pixel(
                ppu_state.is_background_enabled(),
                ppu_state.is_obj_enabled(),
                color_palettes,
            ));
        }

        self.fifos.shift();
    }
}

impl Renderer for CgbRenderer {
    type Vram = CgbVram;
    type Extra = ColorPalettes;
    type ScanlineBuilder = ArrayVec<u16, 160>;

    fn new(objects: ArrayVec<Sprite, 10>) -> Self {
        Self::new(objects)
    }

    fn execute(
        &mut self,
        window_y: &mut Option<u8>,
        ppu_state: &PpuState<Self::Vram>,
        extra: &Self::Extra,
        ly: u8,
        cycle: u64,
    ) {
        // we don't have to care about color palettes here since the render pixel function will just ignore them if it's not needed
        self.execute(window_y, ppu_state, extra, ly, cycle);
    }

    fn get_scanline_builder(&self) -> &Self::ScanlineBuilder {
        &self.scanline
    }
}

#[derive(Clone)]
pub struct RenderingState {
    pub is_shifting: bool,
    pub is_sprite_fetching_enable: bool,
}

#[cfg(test)]
mod tests {
    use arrayvec::ArrayVec;

    use crate::{
        WIDTH,
        ppu::{LcdControl, PpuState, Sprite, TileAttributes, renderer::DmgRenderer},
    };

    // all timings are +2 compared to pandocs timings
    const MINIMUM_TIME: u16 = 174;

    fn get_timing(
        mut window_y: Option<u8>,
        objects: ArrayVec<Sprite, 10>,
        ppu_state: &PpuState,
        ly: u8,
    ) -> u16 {
        let mut renderer = DmgRenderer::new(objects);
        let mut dots = 0;
        while renderer.scanline.len() < WIDTH {
            renderer.execute(&mut window_y, ppu_state, ly, 0);
            dots += 1;
        }
        dots
    }

    #[test]
    fn normal_timing() {
        assert_eq!(
            get_timing(None, Default::default(), &Default::default(), 0),
            MINIMUM_TIME
        );
    }

    #[test]
    fn with_window() {
        // https://gbdev.io/pandocs/Scrolling.html#ff4aff4b--wy-wx-window-y-position-x-position-plus-7
        // Citation: The Window is visible (if enabled) when both coordinates are in the ranges WX=0..166, WY=0..143 respectively
        for wx in 0..167 {
            // https://gbdev.io/pandocs/Rendering.html#mode-3-length
            // Citation: After the last non-window pixel is emitted, a 6-dot penalty is incurred
            assert_eq!(
                get_timing(
                    Some(0),
                    Default::default(),
                    &PpuState {
                        lcd_control: LcdControl::WINDOW_ENABLE,
                        old_lcd_control: LcdControl::WINDOW_ENABLE,
                        wx,
                        old_wx: wx,
                        ..Default::default()
                    },
                    0
                ),
                MINIMUM_TIME + 6,
                "Bad timing with WX = {wx}"
            );
        }

        // the window is not visible
        assert_eq!(
            get_timing(
                Some(0),
                Default::default(),
                &PpuState {
                    lcd_control: LcdControl::WINDOW_ENABLE,
                    wx: 167,
                    old_wx: 167,
                    ..Default::default()
                },
                0
            ),
            MINIMUM_TIME,
            "Bad timing with WX = 167"
        );
    }

    #[test]
    fn with_objects() {
        let objects = ArrayVec::from_iter([Sprite {
            flags: TileAttributes::empty(),
            tile_index: 0,
            x: 0,
            y: 0,
        }]);
        // https://gbdev.io/pandocs/Rendering.html#obj-penalty-algorithm
        // Citation: an OBJ with an OAM X position of 0 always incurs a 11-dot penalty
        assert_eq!(
            get_timing(
                None,
                objects,
                &PpuState {
                    lcd_control: LcdControl::OBJ_ENABLE,
                    ..Default::default()
                },
                0
            ),
            MINIMUM_TIME + 11
        );
    }

    #[test]
    fn with_scroll_x() {
        for scx in 0..=u8::MAX {
            // https://gbdev.io/pandocs/Rendering.html#mode-3-length
            // Citation: At the very beginning of Mode 3, rendering is paused for SCX % 8 dots
            assert_eq!(
                get_timing(
                    None,
                    Default::default(),
                    &PpuState {
                        scx,
                        ..Default::default()
                    },
                    0
                ),
                MINIMUM_TIME + u16::from(scx % 8),
                "Failed with scx {scx}"
            );
        }
    }
}
