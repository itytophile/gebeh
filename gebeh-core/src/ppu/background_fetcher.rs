use crate::{
    addresses::VIDEO_RAM,
    ppu::{
        Scrolling, TILE_LENGTH, Tile, TileAttributes, TileVram,
        fifos::{CgbFifos, DmgFifos},
        renderer::RenderingState,
        vram::VRAM_BANK_SIZE,
    },
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
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        rendering_state: &mut RenderingState,
        fifos: &mut DmgFifos,
        vram: &[u8; VRAM_BANK_SIZE],
        tile_map_address: u16,
        scrolling: Scrolling,
        y: u8,
        is_signed_addressing: bool,
    ) {
        use BackgroundFetcherStep::*;
        if let Ready(tile) = self.step {
            if !fifos.is_background_empty() {
                return;
            }
            fifos.replace_background(tile);
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

#[derive(Clone, Copy, Default)]
pub enum CgbBackgroundFetcherStep {
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
        attribute: TileAttributes,
        scy: Option<u8>,
    },
    FetchingTileHigh {
        tile_index: u8,
        attribute: TileAttributes,
        tile_low: u8,
        scy: Option<u8>,
    },
    Ready {
        tile_line: [u8; 2],
        attribute: TileAttributes,
    },
}

// background and window to be precise
#[derive(Clone, Default)]
pub struct CgbBackgroundFetcher {
    pub step: CgbBackgroundFetcherStep,
    pub x: u8, // will be used like x.max(1) - 1 thus 0 is the dummy fetch
}

impl CgbBackgroundFetcher {
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        rendering_state: &mut RenderingState,
        fifos: &mut CgbFifos,
        vram_banks: &[[u8; VRAM_BANK_SIZE]; 2],
        tile_map_address: u16,
        scrolling: Scrolling,
        y: u8,
        is_signed_addressing: bool,
    ) {
        use CgbBackgroundFetcherStep::*;
        if let Ready {
            tile_line: tile,
            attribute,
        } = self.step
        {
            if !fifos.is_background_empty() {
                return;
            }
            fifos.replace_background(
                if attribute.contains(TileAttributes::X_FLIP) {
                    [tile[0].reverse_bits(), tile[1].reverse_bits()]
                } else {
                    tile
                },
                attribute,
            );
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
                let address = usize::from(address - VIDEO_RAM);
                FetchingTileLow {
                    tile_index: vram_banks[0][address],
                    attribute: TileAttributes::from_bits_retain(vram_banks[1][address]),
                    scy: None,
                }
            }
            FetchingTileLow {
                scy: None,
                tile_index,
                attribute,
            } => FetchingTileLow {
                tile_index,
                attribute,
                scy: Some(scrolling.y),
            },
            FetchingTileLow {
                tile_index,
                attribute,
                scy: Some(scy),
            } => {
                let tile = get_bg_win_tile(
                    vram_banks[usize::from(attribute.contains(TileAttributes::CGB_BANK))][..0x1800]
                        .try_into()
                        .unwrap(),
                    tile_index,
                    is_signed_addressing,
                );
                let line = (usize::from(y) + usize::from(scy)) % 8;
                let line = if attribute.contains(TileAttributes::Y_FLIP) {
                    7 - line
                } else {
                    line
                };
                FetchingTileHigh {
                    tile_index,
                    tile_low: tile[2 * line],
                    attribute,
                    scy: None,
                }
            }
            FetchingTileHigh {
                scy: None,
                tile_index,
                tile_low,
                attribute,
            } => FetchingTileHigh {
                tile_index,
                tile_low,
                attribute,
                scy: Some(scrolling.y),
            },
            FetchingTileHigh {
                tile_index,
                tile_low,
                attribute,
                scy: Some(scy),
            } => {
                // sprite fetcher can start fetching one cycle before the end of background fetching
                rendering_state.is_sprite_fetching_enable = true;
                let tile = get_bg_win_tile(
                    vram_banks[usize::from(attribute.contains(TileAttributes::CGB_BANK))][..0x1800]
                        .try_into()
                        .unwrap(),
                    tile_index,
                    is_signed_addressing,
                );
                self.x += 1;
                let line = (usize::from(y) + usize::from(scy)) % 8;
                let line = if attribute.contains(TileAttributes::Y_FLIP) {
                    7 - line
                } else {
                    line
                };
                Ready {
                    tile_line: [tile_low, tile[2 * line + 1]],
                    attribute,
                }
            }
            sleeping => sleeping,
        };
    }
}
