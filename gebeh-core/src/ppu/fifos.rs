use crate::ppu::{
    color::{ColorIndex, DmgColor},
    color_palettes::ColorPalettes,
};

// according to https://www.reddit.com/r/EmuDev/comments/s6cpis/comment/ht3lcfq/
#[derive(Default, Clone)]
pub struct Fifos {
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

impl Fifos {
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

    pub fn load_sprite(&mut self, tile: [u8; 2], priority: bool, palette: bool) {
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

    pub fn is_background_empty(&self) -> bool {
        self.background_pixels_count == 0
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
}

#[derive(Default, Clone)]
pub struct CgbFifos {
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
    // 8 x 3-bits palettes
    sprite_palette: u32,
    current_background_palette: u8,
    background_pixels_count: u8,
    shifted_count: u8,
}

impl CgbFifos {
    pub fn shift(&mut self) {
        self.bg0 >>= 1;
        self.bg1 >>= 1;
        self.sp0 >>= 1;
        self.sp1 >>= 1;
        self.mask >>= 1;
        self.sprite_palette >>= 3;

        self.background_pixels_count -= 1;
        self.shifted_count = self.shifted_count.wrapping_add(1);
    }

    fn scale_mask_by_3(mask: u8) -> u32 {
        let mut scaled = 0;

        for index in 0..8 {
            if mask & (1 << index) != 0 {
                scaled |= 0x07 << (index * 3);
            }
        }

        scaled
    }

    pub fn load_sprite(&mut self, tile: [u8; 2], priority: bool, palette: u8) {
        let existing_sprite_mask = self.sp0 | self.sp1;
        // we must keep the existing sprite so we unset the bits already present from the new mask
        let new_sprite_mask = (tile[0] | tile[1]) & !existing_sprite_mask;
        if priority {
            self.mask |= new_sprite_mask;
        } else {
            self.mask &= !new_sprite_mask;
        }

        for index in 0..8 {
            if new_sprite_mask & (1 << index) != 0 {
                let shift = index * 3;
                self.sprite_palette =
                    self.sprite_palette & !(0x07 << shift) | u32::from(palette) << shift;
            }
        }

        self.sp0 = new_sprite_mask & tile[0] | !new_sprite_mask & self.sp0;
        self.sp1 = new_sprite_mask & tile[1] | !new_sprite_mask & self.sp1;
    }

    pub fn replace_background(&mut self, tile: [u8; 2], background_palette: u8) {
        self.bg0 = tile[0];
        self.bg1 = tile[1];
        self.background_pixels_count = 8;
        self.current_background_palette = background_palette;
    }

    pub fn render_pixel(
        &self,
        is_background_enabled: bool,
        is_obj_enabled: bool,
        color_palettes: &ColorPalettes,
    ) -> u16 {
        let bg_color_index = if is_background_enabled {
            ColorIndex::new(self.bg0 & 1 != 0, self.bg1 & 1 != 0)
        } else {
            ColorIndex::Zero
        };
        let sp_color_index = if is_obj_enabled {
            ColorIndex::new(self.sp0 & 1 != 0, self.sp1 & 1 != 0)
        } else {
            ColorIndex::Zero
        };

        if sp_color_index == ColorIndex::Zero
            || (self.mask & 1 != 0 && bg_color_index != ColorIndex::Zero)
        {
            return color_palettes
                .background
                .get_palette(self.current_background_palette)
                [usize::from(u8::from(bg_color_index))];
        }

        color_palettes
            .objects
            .get_palette(u8::try_from(self.sprite_palette & 0x07).unwrap())
            [usize::from(u8::from(sp_color_index))]
    }

    pub fn is_background_empty(&self) -> bool {
        self.background_pixels_count == 0
    }

    pub fn reset_background(&mut self) {
        self.background_pixels_count = 0;
    }

    pub fn get_shifted_count(&self) -> u8 {
        self.shifted_count
    }
}
