use arrayvec::ArrayVec;
use bitfield_struct::bitfield;

use crate::ppu::{
    TileAttributes,
    color::{ColorIndex, DmgColor},
    color_palettes::ColorPalettes,
};

// according to https://www.reddit.com/r/EmuDev/comments/s6cpis/comment/ht3lcfq/
#[derive(Default, Clone)]
pub struct DmgFifos {
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
    background_pixels_count: u8,
    shifted_count: u8,
}

impl DmgFifos {
    pub fn shift(&mut self) {
        self.bg0 <<= 1;
        self.bg1 <<= 1;
        self.sp0 <<= 1;
        self.sp1 <<= 1;
        self.mask <<= 1;
        self.palette <<= 1;

        self.background_pixels_count -= 1;
        self.shifted_count = self.shifted_count.wrapping_add(1);
    }

    pub fn replace_background(&mut self, tile: [u8; 2]) {
        self.bg0 = tile[0];
        self.bg1 = tile[1];
        self.background_pixels_count = 8;
    }

    pub fn render_pixel(
        &self,
        bgp: u8,
        obp0: u8,
        obp1: u8,
        is_background_enabled: bool,
        is_obj_enabled: bool,
    ) -> DmgColor {
        let bg_color_index = if is_background_enabled {
            ColorIndex::new(self.bg0 & 0x80 != 0, self.bg1 & 0x80 != 0)
        } else {
            ColorIndex::Zero
        };
        let sp_color_index = if is_obj_enabled {
            ColorIndex::new(self.sp0 & 0x80 != 0, self.sp1 & 0x80 != 0)
        } else {
            ColorIndex::Zero
        };

        if sp_color_index == ColorIndex::Zero
            || (self.mask & 0x80 != 0 && bg_color_index != ColorIndex::Zero)
        {
            return bg_color_index.get_color(bgp);
        }

        sp_color_index.get_color(if self.palette & 0x80 != 0 { obp1 } else { obp0 })
    }

    pub fn reset_background(&mut self) {
        self.background_pixels_count = 0;
    }

    pub fn get_shifted_count(&self) -> u8 {
        self.shifted_count
    }

    pub fn insert_window_reactivation_pixel(&mut self) {
        self.bg0 >>= 1;
        self.bg1 >>= 1;
        self.background_pixels_count = 8.min(self.background_pixels_count + 1);
    }

    pub fn load_sprite(&mut self, tile: [u8; 2], attributes: TileAttributes) {
        let existing_sprite_mask = self.sp0 | self.sp1;
        // we must keep the existing sprite so we unset the bits already present from the new mask
        let new_sprite_mask = (tile[0] | tile[1]) & !existing_sprite_mask;
        if attributes.contains(TileAttributes::PRIORITY) {
            self.mask |= new_sprite_mask;
        } else {
            self.mask &= !new_sprite_mask;
        }
        if attributes.contains(TileAttributes::DMG_PALETTE) {
            self.palette |= new_sprite_mask;
        } else {
            self.palette &= !new_sprite_mask;
        }
        self.sp0 = new_sprite_mask & tile[0] | !new_sprite_mask & self.sp0;
        self.sp1 = new_sprite_mask & tile[1] | !new_sprite_mask & self.sp1;
    }

    pub fn is_background_empty(&self) -> bool {
        self.background_pixels_count == 0
    }
}

#[bitfield(u16)]
struct PixelInfo {
    // if the background must be drawn over the sprite
    priority: bool,
    #[bits(3)]
    palette: u8,
    #[bits(4)]
    oam_index: u8,
    #[bits(2)]
    color_index: u8,
    #[bits(6)]
    _padding: u8,
}

#[derive(Default, Clone)]
pub struct CgbFifos {
    // for low background tile data
    bg0: u8,
    // for high background tile data
    bg1: u8,
    sprite_pixels: ArrayVec<PixelInfo, 8>,
    current_background_attributes: TileAttributes,
    background_pixels_count: u8,
    shifted_count: u8,
}

pub struct DmgPalettes {
    pub bgp: u8,
    pub obp: [u8; 2],
}

impl CgbFifos {
    pub fn shift(&mut self) {
        self.bg0 <<= 1;
        self.bg1 <<= 1;
        self.sprite_pixels.pop();

        self.background_pixels_count -= 1;
        self.shifted_count = self.shifted_count.wrapping_add(1);
    }

    pub fn replace_background(&mut self, tile: [u8; 2], attributes: TileAttributes) {
        self.bg0 = tile[0];
        self.bg1 = tile[1];
        self.background_pixels_count = 8;
        self.current_background_attributes = attributes;
    }

    pub fn render_pixel(
        &self,
        master_background_priority: bool,
        is_obj_enabled: bool,
        color_palettes: &ColorPalettes,
        dmg_palettes: Option<DmgPalettes>,
    ) -> u16 {
        let bg_color_index = ColorIndex::new(self.bg0 & 0x80 != 0, self.bg1 & 0x80 != 0);

        let sprite_pixel = self.sprite_pixels.last().copied().unwrap_or_default();

        let sp_color_index = sprite_pixel.color_index();

        let obj_over_bg = !master_background_priority
            || !self
                .current_background_attributes
                .contains(TileAttributes::PRIORITY)
                && !sprite_pixel.priority();

        if is_obj_enabled
            && (obj_over_bg && sp_color_index != 0
                || !obj_over_bg && bg_color_index == ColorIndex::Zero)
        {
            let sp_color_index = if let Some(dmg) = dmg_palettes {
                // stolen from sameboy
                (dmg.obp[usize::from(sprite_pixel.palette())] >> (sp_color_index << 1)) & 3
            } else {
                sp_color_index
            };

            color_palettes.objects.get_palette(sprite_pixel.palette())[usize::from(sp_color_index)]
        } else {
            let bg_color_index = if let Some(dmg) = dmg_palettes {
                // stolen from sameboy
                (dmg.bgp >> (u8::from(bg_color_index) << 1)) & 3
            } else {
                u8::from(bg_color_index)
            };

            color_palettes
                .background
                .get_palette(self.current_background_attributes.get_cgb_palette_index())
                [usize::from(bg_color_index)]
        }
    }

    pub fn reset_background(&mut self) {
        self.background_pixels_count = 0;
    }

    pub fn get_shifted_count(&self) -> u8 {
        self.shifted_count
    }

    pub fn load_sprite(
        &mut self,
        tile: [u8; 2],
        attributes: TileAttributes,
        oam_index: u8,
        is_dmg_style: bool,
        is_dmg_compatible: bool,
    ) {
        let mut new_sprite_pixels: ArrayVec<PixelInfo, 8> = tile_to_indexes(tile)
            .map(|color_index| {
                PixelInfo::new()
                    .with_color_index(color_index)
                    .with_palette(if is_dmg_compatible {
                        attributes.contains(TileAttributes::DMG_PALETTE) as u8
                    } else {
                        attributes.get_cgb_palette_index()
                    })
                    .with_priority(attributes.contains(TileAttributes::PRIORITY))
                    .with_oam_index(oam_index)
            })
            .collect();

        if is_dmg_style {
            for (new, old) in new_sprite_pixels
                .iter_mut()
                .rev()
                .zip(self.sprite_pixels.iter().rev().copied())
            {
                *new = if old.color_index() == 0 && new.color_index() != 0 {
                    *new
                } else {
                    old
                };
            }
        } else {
            for (new, old) in new_sprite_pixels
                .iter_mut()
                .rev()
                .zip(self.sprite_pixels.iter().rev().copied())
            {
                // Citation: In CGB mode, only the object’s location in OAM determines its priority.
                // The earlier the object, the higher its priority.
                *new = match (
                    old.oam_index() < new.oam_index(),
                    new.color_index() == 0,
                    old.color_index() == 0,
                ) {
                    (true, true, _) => old,
                    (_, false, true) => *new,
                    (true, false, false) => old,
                    (false, true, true) => *new,
                    (false, true, false) => old,
                    (false, false, false) => *new,
                }
            }
        }

        self.sprite_pixels = new_sprite_pixels;
    }

    pub fn is_background_empty(&self) -> bool {
        self.background_pixels_count == 0
    }
}

fn tile_to_indexes(tile: [u8; 2]) -> impl Iterator<Item = u8> {
    (0..8).map(move |index| ((tile[0] >> index) & 1) | (((tile[1] >> index) & 1) << 1))
}
