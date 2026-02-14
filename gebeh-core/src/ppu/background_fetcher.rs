use crate::{
    ppu::{Scrolling, TILE_LENGTH, Tile, TileVram, renderer::RenderingState},
    state::VIDEO_RAM,
};

#[derive(Clone, Copy, Default)]
pub enum BackgroundFetcherStep {
    // https://gbdev.io/pandocs/Scrolling.html#scrolling
    // Citation: The scroll registers are re-read on each tile fetch, except for the low 3 bits of SCX
    // scrolling set at 0 when handling window tiles
    #[default]
    WaitingForScrollRegisters,
    // no delay for him because we have the beautiful WaitingForScrollRegisters
    FetchingTileIndex {
        scy: u8,
        scx: u8,
    },
    FetchingTileLow {
        tile_index: u8,
        scy: Option<u8>,
    },
    FetchingTileHigh {
        tile_index: u8,
        tile_low: u8,
        scy: Option<u8>,
    },
    Ready([u8; 2]),
}

// background and window to be precise
#[derive(Clone, Default)]
pub struct BackgroundFetcher {
    pub step: BackgroundFetcherStep,
    pub x: u8, // will be used like x.max(1) - 1 thus 0 is the dummy fetch
}

impl BackgroundFetcher {
    pub fn execute(
        &mut self,
        rendering_state: &mut RenderingState,
        vram: &[u8; 0x2000],
        tile_map_address: u16,
        scrolling: Scrolling,
        y: u8,
        is_signed_addressing: bool,
    ) {
        use BackgroundFetcherStep::*;
        if let Ready(tile) = self.step {
            if !rendering_state.fifos.is_background_empty() {
                return;
            }
            rendering_state.fifos.replace_background(tile);
            // we will start another fetching process, too bad for the sprite fetcher
            rendering_state.is_sprite_fetching_enable = false;
            self.step = WaitingForScrollRegisters;
        }
        self.step = match self.step {
            WaitingForScrollRegisters => FetchingTileIndex {
                scy: scrolling.y,
                scx: scrolling.x,
            },
            FetchingTileIndex { scy, scx } => {
                let address = tile_map_address
                    + u16::from((self.x.max(1) - 1 + scx / 8) & 0x1f)
                    + 32 * u16::from(y.wrapping_add(scy) / 8); // don't simplify 32 / 8 to 4
                FetchingTileLow {
                    tile_index: vram[usize::from(address - VIDEO_RAM)],
                    scy: None,
                }
            }
            FetchingTileLow {
                scy: None,
                tile_index,
            } => FetchingTileLow {
                tile_index,
                scy: Some(scrolling.y),
            },
            FetchingTileLow {
                tile_index,
                scy: Some(scy),
            } => {
                let tile = get_bg_win_tile(
                    vram[..0x1800].try_into().unwrap(),
                    tile_index,
                    is_signed_addressing,
                );
                FetchingTileHigh {
                    tile_index,
                    tile_low: tile[2 * ((usize::from(y) + usize::from(scy)) % 8)],
                    scy: None,
                }
            }
            FetchingTileHigh {
                scy: None,
                tile_index,
                tile_low,
            } => FetchingTileHigh {
                tile_index,
                tile_low,
                scy: Some(scrolling.y),
            },
            FetchingTileHigh {
                tile_index,
                tile_low,
                scy: Some(scy),
            } => {
                // sprite fetcher can start fetching one cycle before the end of background fetching
                rendering_state.is_sprite_fetching_enable = true;
                let tile = get_bg_win_tile(
                    vram[..0x1800].try_into().unwrap(),
                    tile_index,
                    is_signed_addressing,
                );
                self.x += 1;
                Ready([
                    tile_low,
                    tile[2 * ((usize::from(y) + usize::from(scy)) % 8) + 1],
                ])
            }
            sleeping => sleeping,
        };
    }
}

#[must_use]
pub fn get_bg_win_tile(vram: &TileVram, index: u8, is_signed_addressing: bool) -> &Tile {
    let base = if is_signed_addressing {
        0x1000usize.strict_add_signed(isize::from(index.cast_signed()) * isize::from(TILE_LENGTH))
    } else {
        usize::from(index) * usize::from(TILE_LENGTH)
    };
    vram[base..base + usize::from(TILE_LENGTH)]
        .try_into()
        .unwrap()
}
