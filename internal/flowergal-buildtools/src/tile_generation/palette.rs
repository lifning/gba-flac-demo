// Copyright (C) 2021 lifning, licensed under the GNU Affero General Public License version 3.

use super::Pattern;

pub const PAL_LEN: usize = 16;

pub struct Palette {
    pub colors: Vec<gba::Color>,
    pub max_colors: usize,
}

impl Palette {
    pub(crate) fn new(max_colors: usize) -> Self {
        Palette {
            colors: vec![gba::Color::from_rgb(0, 0, 0)], // backdrop color if first palette
            max_colors,
        }
    }

    pub fn find_existing(&self, pixel_colors: &[gba::Color]) -> Option<Pattern> {
        let mut tile_pixels = Vec::new();
        for color in pixel_colors {
            if (color.0 & (1 << 15)) == 0 {
                tile_pixels.push(0) // transparent is always index 0
            } else if let Some(index) = self.colors.iter().position(|c| *c == *color) {
                tile_pixels.push(index)
            } else {
                return None;
            }
        }
        Some(Pattern(tile_pixels))
    }

    pub fn try_onboard_tile(&mut self, pixel_colors: &[gba::Color]) -> Option<Pattern> {
        let start_len = self.colors.len(); // revert to this if we abandon
        let mut tile_pixels = Vec::new(); // will convert to 4bpp later
        for color in pixel_colors {
            if let Some(index) = self.try_onboard_color(*color) {
                tile_pixels.push(index);
            } else {
                // drop any colors we tentatively added, since we can't represent this tile
                self.colors.truncate(start_len);
                return None;
            }
        }
        Some(Pattern(tile_pixels))
    }

    pub fn try_onboard_color(&mut self, color: gba::Color) -> Option<usize> {
        // if non-transparent
        if (color.0 & (1 << 15)) != 0 {
            if let Some(index) = self.colors.iter().position(|c| *c == color) {
                Some(index)
            } else if self.colors.len() < self.max_colors {
                let last = self.colors.len();
                self.colors.push(color);
                Some(last)
            } else {
                None
            }
        } else {
            Some(0) // transparent is entry 0 in all palettes
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PalBankId {
    Plain(usize),
    Blended(usize),
}

impl PalBankId {
    pub fn new(palbank: usize, blend: bool) -> Self {
        if blend {
            PalBankId::Blended(palbank)
        } else {
            PalBankId::Plain(palbank)
        }
    }

    pub fn bake(&self, pal_bank_ofs: usize) -> u16 {
        match self {
            PalBankId::Plain(x) => (pal_bank_ofs + x) as u16,
            PalBankId::Blended(x) => *x as u16,
        }
    }
}

pub struct PaletteBank {
    pub palettes_blend: Vec<Palette>,
    pub palettes_plain: Vec<Palette>,
    pub max_palettes: usize,
    pub palette_max_colors: usize,
}

impl PaletteBank {
    pub fn new(max_colors: usize) -> Self {
        PaletteBank {
            palettes_blend: Vec::new(),
            palettes_plain: Vec::new(),
            max_palettes: max_colors / PAL_LEN,
            palette_max_colors: PAL_LEN,
        }
    }

    pub fn try_onboard_colors(&mut self, pixel_colors: &[gba::Color], blend: bool) -> Option<(Pattern, PalBankId)> {
        /*
        if let Some(x) = self.find_existing_colors(pixel_colors, blend) {
            return Some(x)
        }
        */
        let total_palettes = self.palettes_plain.len() + self.palettes_blend.len();
        let palette_group = if blend { &mut self.palettes_blend } else { &mut self.palettes_plain };
        for (palbank, pal) in palette_group.iter_mut().enumerate() {
            if let Some(img) = pal.try_onboard_tile(pixel_colors) {
                return Some((img, PalBankId::new(palbank, blend)));
            }
        }
        if total_palettes < self.max_palettes {
            let mut pal = Palette::new(self.palette_max_colors);
            if let Some(img) = pal.try_onboard_tile(pixel_colors) {
                let palbank = palette_group.len();
                palette_group.push(pal);
                return Some((img, PalBankId::new(palbank, blend)));
            }
        }
        None
    }

    /*
    fn find_existing_colors(&self, pixel_colors: &[gba::Color], blend: bool) -> Option<(Pattern, PalBankId)> {
        let pal_set = if blend { &self.palettes_blend } else { &self.palettes_plain };
        for (palbank, pal) in pal_set.iter().enumerate() {
            if let Some(img) = pal.find_existing(pixel_colors) {
                return Some((img, PalBankId::new(palbank, blend)));
            }
        }
        None
    }
    */
}
