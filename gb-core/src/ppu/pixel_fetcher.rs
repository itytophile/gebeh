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

use core::num::{NonZero, NonZeroU8};

use arrayvec::ArrayVec;

use crate::{
    ppu::{
        Color, ColorIndex, Either, LcdControl, ObjectAttribute, ObjectFlags, get_bg_win_tile,
        get_line_from_tile, get_object_tile,
    },
    state::{Scrolling, State, VIDEO_RAM},
};

// when rust will have effects or generators or whatever
#[derive(Clone, Copy)]
pub enum PixelFetcherStep {
    // https://gbdev.io/pandocs/Scrolling.html#scrolling
    // Citation: The scroll registers are re-read on each tile fetch, except for the low 3 bits of SCX
    // scrolling set at 0 when handling window tiles
    WaitingForScrollRegisters,
    // no delay for him because we have the beautiful WaitingForScrollRegisters
    FetchingTileIndex {
        scx: u8,
        scy: u8,
    },
    FetchingTileLow {
        one_dot_delay: bool,
        tile_index: u8,
        scy: u8,
    },
    FetchingTileHigh {
        one_dot_delay: bool,
        tile_index: u8,
        tile_low: u8,
        scy: u8,
    },
}

#[derive(Clone, Copy)]
pub struct PixelFetcher {
    // offset the address used to read the tile index from the tilemap
    // incremented when we push pixels to the FIFO
    // Don't forget that we can't have background pixels right (greater x) to
    // the window (window is always displayed after or over the background)
    x: u8,
    step: PixelFetcherStep,
}

impl Default for PixelFetcher {
    fn default() -> Self {
        Self {
            x: 0,
            step: PixelFetcherStep::WaitingForScrollRegisters,
        }
    }
}

impl PixelFetcher {
    #[must_use]
    pub fn next(
        mut self,
        vram: &[u8; 0x2000],
        tile_map_address: u16,
        scrolling: Scrolling,
        y: u8,
        is_signed_addressing: bool,
    ) -> Either<Self, ReadyPixelFetcher> {
        use PixelFetcherStep::*;
        self.step = match self.step {
            WaitingForScrollRegisters => FetchingTileIndex {
                scx: scrolling.x,
                scy: scrolling.y,
            },
            FetchingTileIndex { scx, scy } => {
                let address = tile_map_address
                    + u16::from((self.x + scx / 8) & 0x1f)
                    + 32 * (((u16::from(y) + u16::from(scy)) & 0xff) / 8); // don't simplify 32 / 8 to 4
                FetchingTileLow {
                    one_dot_delay: false,
                    tile_index: vram[usize::from(address - VIDEO_RAM)],
                    scy,
                }
            }
            FetchingTileLow {
                one_dot_delay: false,
                tile_index,
                scy,
            } => FetchingTileLow {
                one_dot_delay: true,
                tile_index,
                scy,
            },
            FetchingTileLow {
                one_dot_delay: true,
                tile_index,
                scy,
            } => {
                let tile = get_bg_win_tile(
                    vram[..0x1800].try_into().unwrap(),
                    tile_index,
                    is_signed_addressing,
                );
                FetchingTileHigh {
                    one_dot_delay: false,
                    tile_index,
                    tile_low: tile[2 * ((usize::from(y) + usize::from(scy)) % 8)],
                    scy,
                }
            }
            FetchingTileHigh {
                one_dot_delay: false,
                tile_index,
                tile_low,
                scy,
            } => FetchingTileHigh {
                one_dot_delay: true,
                tile_index,
                tile_low,
                scy,
            },
            FetchingTileHigh {
                one_dot_delay: true,
                tile_index,
                tile_low,
                scy,
            } => {
                let tile = get_bg_win_tile(
                    vram[..0x1800].try_into().unwrap(),
                    tile_index,
                    is_signed_addressing,
                );
                return Either::Right(ReadyPixelFetcher {
                    x: self.x,
                    tile_line: [
                        tile_low,
                        tile[2 * ((usize::from(y) + usize::from(scy)) % 8) + 1],
                    ],
                });
            }
        };

        Either::Left(self)
    }
}

#[derive(Clone, Copy)]
pub struct ReadyPixelFetcher {
    x: u8,
    pub tile_line: [u8; 2],
}

impl ReadyPixelFetcher {
    pub fn consume(self) -> PixelFetcher {
        PixelFetcher {
            step: PixelFetcherStep::WaitingForScrollRegisters,
            x: self.x + 1,
        }
    }
}

// 2

struct Renderer {
    background_pixel_fetcher: BackgroundPixelFetcher,
    sprite_pixel_fetcher: SpritePixelFetcher,
    state: RenderingState,
}

impl Renderer {
    fn new() -> Self {
        Self {
            state: RenderingState {
                // We begin the rendering at x = -8, so we have to discard those negative pixels
                is_lcd_accepting_pixels: false,
                is_shifting: false,
                // will be disabled right away by the first background fetch
                is_sprite_fetching_enable: true,
                fifos: Default::default(),
            },
            background_pixel_fetcher: Default::default(),
            sprite_pixel_fetcher: Default::default(),
        }
    }

    fn execute(&mut self, scanline: &mut ArrayVec<Color, 160>, state: &State) {
        // those systems can run "concurrently"
        self.background_pixel_fetcher.execute(&mut self.state);
        self.sprite_pixel_fetcher.execute(&mut self.state);
        if self.state.is_lcd_accepting_pixels {
            scanline.push(self.state.fifos.render_pixel(
                state.bgp_register,
                state.obp0,
                state.obp1,
            ));
        }
        if self.state.is_shifting {
            self.state.fifos.shift();
        }
    }
}

// according to https://www.reddit.com/r/EmuDev/comments/s6cpis/comment/ht3lcfq/
#[derive(Default)]
struct Fifos {
    // for low background tile data
    bg0: u8,
    // for high background tile data
    bg1: u8,
    // for low sprite tile data
    sp0: u8,
    // for high sprite tile data
    sp1: u8,
    // if the background must be drawn over the sprite
    mask: u8,
    // sprite palette, the background palette is checked globally before pushing to the LCD
    palette: u8,
    // to know if the fifo is empty
    shifted_count: u8,
}

impl Fifos {
    fn shift(&mut self) {
        self.bg0 <<= 1;
        self.bg1 <<= 1;
        self.sp0 <<= 1;
        self.sp1 <<= 1;
        self.mask <<= 1;
        self.palette <<= 1;
        self.shifted_count = self.shifted_count.wrapping_add(1);
    }

    fn load_sprite(&mut self, tile: [u8; 2], priority: bool, palette: bool) {
        let existing_sprite_mask = self.sp0 | self.sp1;
        // we must keep the existing sprite so we unset the bits already present from the new mask
        let new_sprite_mask = (tile[0] | tile[1]) & !existing_sprite_mask;
        if priority {
            self.mask |= new_sprite_mask;
        } else {
            self.mask &= !new_sprite_mask;
        }
        if palette {
            self.palette |= new_sprite_mask;
        } else {
            self.palette &= !new_sprite_mask;
        }
        self.sp0 = new_sprite_mask & tile[0] | !new_sprite_mask & self.sp0;
        self.sp1 = new_sprite_mask & tile[1] | !new_sprite_mask & self.sp1;
    }

    fn replace_background(&mut self, tile: [u8; 2]) {
        self.bg0 = tile[0];
        self.bg1 = tile[1];
    }

    fn render_pixel(&self, bgp: u8, obp0: u8, obp1: u8) -> Color {
        let bg_color_index = ColorIndex::new(self.bg0 & 0x80 != 0, self.bg1 & 0x80 != 0);
        let sp_color_index = ColorIndex::new(self.sp0 & 0x80 != 0, self.sp1 & 0x80 != 0);

        if sp_color_index == ColorIndex::Zero
            || (self.mask & 0x80 != 0 && bg_color_index != ColorIndex::Zero)
        {
            return bg_color_index.get_color(bgp);
        }

        sp_color_index.get_color(if self.palette & 0x80 != 0 { obp1 } else { obp0 })
    }

    fn is_background_empty(&self) -> bool {
        self.shifted_count.is_multiple_of(8)
    }
}

struct RenderingState {
    is_shifting: bool,
    is_lcd_accepting_pixels: bool,
    is_sprite_fetching_enable: bool,
    fifos: Fifos,
}

#[derive(Clone, Copy)]
pub enum BackgroundPixelFetcherStep {
    // https://gbdev.io/pandocs/Scrolling.html#scrolling
    // Citation: The scroll registers are re-read on each tile fetch, except for the low 3 bits of SCX
    // scrolling set at 0 when handling window tiles
    WaitingForScrollRegisters,
    // no delay for him because we have the beautiful WaitingForScrollRegisters
    FetchingTileIndex {
        scx: u8,
        scy: u8,
    },
    FetchingTileLow {
        one_dot_delay: bool,
        tile_index: u8,
        scy: u8,
    },
    FetchingTileHigh {
        one_dot_delay: bool,
        tile_index: u8,
        tile_low: u8,
        scy: u8,
    },
    Ready([u8; 2]),
}

// background and window to be precise
struct BackgroundPixelFetcher {
    step: BackgroundPixelFetcherStep,
    x: u8, // will be used like x.max(1) - 0 thus 0 is the dummy fetch
}

impl Default for BackgroundPixelFetcher {
    fn default() -> Self {
        Self {
            step: BackgroundPixelFetcherStep::WaitingForScrollRegisters,
            x: 0,
        }
    }
}

impl BackgroundPixelFetcher {
    fn execute(
        &mut self,
        rendering_state: &mut RenderingState,
        vram: &[u8; 0x2000],
        tile_map_address: u16,
        scrolling: Scrolling,
        y: u8,
        is_signed_addressing: bool,
    ) {
        use BackgroundPixelFetcherStep::*;
        if let Ready(tile) = self.step {
            if !rendering_state.fifos.is_background_empty() {
                return;
            }
            rendering_state.fifos.replace_background(tile);
            // we enable it here to start the very first shifting process for the "dummy tile"
            rendering_state.is_shifting = true;
            // we will start another fetching process, too bad for the sprite fetcher
            rendering_state.is_sprite_fetching_enable = false;
            self.step = WaitingForScrollRegisters;
        }
        self.step = match self.step {
            WaitingForScrollRegisters => FetchingTileIndex {
                scx: scrolling.x,
                scy: scrolling.y,
            },
            FetchingTileIndex { scx, scy } => {
                let address = tile_map_address
                    + u16::from((self.x.max(1) - 1 + scx / 8) & 0x1f)
                    + 32 * (((u16::from(y) + u16::from(scy)) & 0xff) / 8); // don't simplify 32 / 8 to 4
                FetchingTileLow {
                    one_dot_delay: false,
                    tile_index: vram[usize::from(address - VIDEO_RAM)],
                    scy,
                }
            }
            FetchingTileLow {
                one_dot_delay: false,
                tile_index,
                scy,
            } => FetchingTileLow {
                one_dot_delay: true,
                tile_index,
                scy,
            },
            FetchingTileLow {
                one_dot_delay: true,
                tile_index,
                scy,
            } => {
                let tile = get_bg_win_tile(
                    vram[..0x1800].try_into().unwrap(),
                    tile_index,
                    is_signed_addressing,
                );
                FetchingTileHigh {
                    one_dot_delay: false,
                    tile_index,
                    tile_low: tile[2 * ((usize::from(y) + usize::from(scy)) % 8)],
                    scy,
                }
            }
            FetchingTileHigh {
                one_dot_delay: false,
                tile_index,
                tile_low,
                scy,
            } => FetchingTileHigh {
                one_dot_delay: true,
                tile_index,
                tile_low,
                scy,
            },
            FetchingTileHigh {
                one_dot_delay: true,
                tile_index,
                tile_low,
                scy,
            } => {
                // sprite fetcher can start fetching one cycle before the end of background fecthing
                rendering_state.is_sprite_fetching_enable = true;
                let tile = get_bg_win_tile(
                    vram[..0x1800].try_into().unwrap(),
                    tile_index,
                    is_signed_addressing,
                );
                Ready([
                    tile_low,
                    tile[2 * ((usize::from(y) + usize::from(scy)) % 8) + 1],
                ])
            }
            sleeping => sleeping,
        };
    }
}

enum SpritePixelFetcherStep {
    // we have access to the object tile_index so it's useless to have it here
    FetchingTileLow { delay: u8 },
    FetchingTileHigh { one_dot_delay: bool, tile_low: u8 },
}

struct SpritePixelFetcher {
    step: SpritePixelFetcherStep,
}

impl Default for SpritePixelFetcher {
    fn default() -> Self {
        Self {
            step: SpritePixelFetcherStep::FetchingTileLow { delay: 0 },
        }
    }
}

impl SpritePixelFetcher {
    // the cursor is in the space as the object coordinates. So cursor = 0 <=> scanline x = -8
    fn execute(
        &mut self,
        rendering_state: &mut RenderingState,
        cursor: u8,
        objects: &mut ArrayVec<ObjectAttribute, 10>,
        state: &State,
    ) {
        let Some(obj) = objects.last() else {
            return;
        };

        if obj.x != cursor || rendering_state.fifos.is_background_empty() {
            return;
        }

        rendering_state.is_shifting = false;
        rendering_state.is_lcd_accepting_pixels = false;

        // stop if background fifo empty to not begin the fetch before the end of the dummy fetch
        if !rendering_state.is_sprite_fetching_enable || rendering_state.fifos.is_background_empty()
        {
            return;
        }

        use SpritePixelFetcherStep::*;

        // 0 -> fetch tile index
        // 1 -> fetch tile index
        // 2 -> fetch tile low
        // 3 -> fetch tile low
        // 4 -> fetch tile high
        // 5 -> fetch tile high (end)

        self.step = match self.step {
            FetchingTileLow { delay: 3 } => FetchingTileHigh {
                one_dot_delay: false,
                tile_low: get_object_tile_line(state, obj)[0],
            },
            FetchingTileLow { delay } => FetchingTileLow { delay: delay + 1 },
            FetchingTileHigh {
                one_dot_delay: false,
                tile_low,
            } => FetchingTileHigh {
                one_dot_delay: true,
                tile_low,
            },
            FetchingTileHigh {
                one_dot_delay: true,
                tile_low,
            } => {
                // we have to fetch the tile line in two steps because the LcdControl::OBJ_SIZE
                // can be changed between fetches (don't know if it works exactly like this)
                let tile_high = get_object_tile_line(state, obj)[1];
                rendering_state.is_shifting = true;
                rendering_state.is_lcd_accepting_pixels = true;
                rendering_state.fifos.load_sprite(
                    [tile_low, tile_high],
                    obj.flags.contains(ObjectFlags::PRIORITY),
                    obj.flags.contains(ObjectFlags::DMG_PALETTE),
                );
                objects.pop();
                FetchingTileLow { delay: 0 }
            }
        };
    }
}

fn get_object_tile_line(state: &State, obj: &ObjectAttribute) -> [u8; 2] {
    let is_big = state.lcd_control.contains(LcdControl::OBJ_SIZE);
    let y_flip = obj.flags.contains(ObjectFlags::Y_FLIP);
    let tile_index = (obj.tile_index & if is_big { 0xfe } else { 0xff })
        + (is_big && (state.ly + 8 >= obj.y) != y_flip) as u8;
    let tile = get_object_tile(
        state.video_ram[usize::from(0x8000 - VIDEO_RAM)..usize::from(0x9000 - VIDEO_RAM)]
            .try_into()
            .unwrap(),
        tile_index,
    );
    let mut y = (state.ly + 16 - obj.y) % 8;
    y = if y_flip { 7 - y } else { y };
    let line = get_line_from_tile(tile, y);
    line
}
