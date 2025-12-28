use arrayvec::ArrayVec;

use crate::{
    ppu::{
        LcdControl, ObjectAttribute, ObjectFlags, get_line_from_tile, get_object_tile,
        renderer::RenderingState,
    },
    state::{State, VIDEO_RAM},
};

#[derive(Clone)]
pub enum SpriteFetcher {
    // we have access to the object tile_index so it's useless to have it here
    FetchingTileLow { delay: u8 },
    FetchingTileHigh { one_dot_delay: bool, tile_low: u8 },
}

impl Default for SpriteFetcher {
    fn default() -> Self {
        Self::FetchingTileLow { delay: 0 }
    }
}

impl SpriteFetcher {
    pub fn execute(
        &mut self,
        // the cursor is in the same "space" as the sprites x coordinates
        // it can be negative if there is some scrolling
        cursor: i16,
        rendering_state: &mut RenderingState,
        objects: &mut ArrayVec<ObjectAttribute, 10>,
        state: &State,
        dots_count: u16,
    ) {
        let Some(obj) = objects.last() else {
            return;
        };

        log::warn!(
            "{dots_count}: obj_x {} cursor {cursor} is_empty {}",
            obj.x,
            rendering_state.fifos.is_background_empty()
        );

        if i16::from(obj.x) != cursor {
            return;
        }

        let is_obj_canceled = !state.lcd_control.contains(LcdControl::OBJ_ENABLE);

        rendering_state.is_shifting = is_obj_canceled;

        if is_obj_canceled {
            objects.pop();
            return;
        }

        // stop if background fifo empty to not begin the fetch before the end of the dummy fetch
        if !rendering_state.is_sprite_fetching_enable || rendering_state.fifos.is_background_empty()
        {
            return;
        }

        log::warn!("{dots_count}: sprite fetching for object at {}", obj.x);

        use SpriteFetcher::*;

        // 0 -> fetch tile index
        // 1 -> fetch tile index
        // 2 -> fetch tile low
        // 3 -> fetch tile low
        // 4 -> fetch tile high
        // 5 -> fetch tile high (end)

        *self = match *self {
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
                // TODO gérer quand plusieurs sprites empilés
                rendering_state.is_shifting = true;
                rendering_state.fifos.load_sprite(
                    if obj.flags.contains(ObjectFlags::X_FLIP) {
                        [tile_low.reverse_bits(), tile_high.reverse_bits()]
                    } else {
                        [tile_low, tile_high]
                    },
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

    get_line_from_tile(tile, y)
}
