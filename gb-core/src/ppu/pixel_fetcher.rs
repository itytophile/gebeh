// relevant docs
// https://github.com/Ashiepaws/GBEDG/blob/97f198d330a51be558aa8fc9f3f0760846d02d95/ppu/index.md#background-pixel-fetching
// https://gbdev.io/pandocs/pixel_fifo.html#fifo-pixel-fetcher
// http://blog.kevtris.org/blogfiles/Nitty%20Gritty%20Gameboy%20VRAM%20Timing.txt

use crate::{
    ppu::{Either, get_bg_win_tile},
    state::{Scrolling, VIDEO_RAM},
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
