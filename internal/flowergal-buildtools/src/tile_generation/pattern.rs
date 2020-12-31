use super::{SbEntry, TILE_H, TILE_W};
use gba::vram::{Tile4bpp, Tile8bpp};
use crate::tile_generation::PalBankId;

// TODO: compare tileimg redundancy on rgbpixels, not pal index images,
//  sort palette colors & *then* map colors

#[derive(Eq, PartialEq)]
pub struct Pattern(pub Vec<usize>);

impl Pattern {
    pub fn hflip(&self) -> Self {
        let mut result = Vec::with_capacity(self.0.len());
        for y in 0..TILE_H {
            for x in (0..TILE_W).rev() {
                result.push(self.0[(y * TILE_W) + x]);
            }
        }
        Pattern(result)
    }
    pub fn vflip(&self) -> Self {
        let mut result = Vec::with_capacity(self.0.len());
        for y in (0..TILE_H).rev() {
            for x in 0..TILE_W {
                result.push(self.0[(y * TILE_W) + x]);
            }
        }
        Pattern(result)
    }
    // from GBATEK:
    // the lower 4 bits define the color for the left (!) dot,
    // the upper 4 bits the color for the right dot.
    pub fn to_4bpp(&self) -> [u8; 8 * 8 / 2] {
        let mut data: [u8; 8 * 8 / 2] = unsafe { std::mem::transmute(Tile4bpp::default()) };
        for (i, x) in self.0.chunks(2).enumerate() {
            let left = x[0];
            let right = x[1];
            data[i] = ((left & 0xF) | ((right & 0xF) << 4)) as u8;
        }
        data
    }
    pub fn to_8bpp(&self) -> [u8; 8 * 8] {
        let mut data: [u8; 8 * 8] = unsafe { std::mem::transmute(Tile8bpp::default()) };
        for (i, x) in self.0.iter().enumerate() {
            data[i] = *x as u8;
        }
        data
    }
}

impl From<&Pattern> for Tile8bpp {
    fn from(img: &Pattern) -> Self {
        Tile8bpp(unsafe { std::mem::transmute(img.to_8bpp()) })
    }
}

impl From<&Pattern> for Tile4bpp {
    fn from(img: &Pattern) -> Self {
        Tile4bpp(unsafe { std::mem::transmute(img.to_4bpp()) })
    }
}

pub struct PatternBank {
    pub patterns: Vec<Pattern>,
    pub max_patterns: usize,
    pub is_4bpp: bool,
}
impl PatternBank {
    pub fn new(is_4bpp: bool) -> Self {
        let max_patterns = if is_4bpp { 512 } else { 256 };
        PatternBank {
            patterns: vec![Pattern(vec![0; TILE_W * TILE_H])],
            max_patterns,
            is_4bpp,
        }
    }

    pub fn find_existing(&self, new_pattern: &Pattern, palbank: PalBankId) -> Option<SbEntry> {
        if let Some(tile_num) = self.patterns.iter().position(|img| *img == *new_pattern) {
            return Some(SbEntry {
                tile_num,
                hflip: false,
                vflip: false,
                palbank,
            });
        }
        if self.is_4bpp {
            let h_flipped = new_pattern.hflip();
            if let Some(tile_num) = self.patterns.iter().position(|img| *img == h_flipped) {
                return Some(SbEntry {
                    tile_num,
                    hflip: true,
                    vflip: false,
                    palbank,
                });
            }
            let v_flipped = new_pattern.vflip();
            if let Some(tile_num) = self.patterns.iter().position(|img| *img == v_flipped) {
                return Some(SbEntry {
                    tile_num,
                    hflip: false,
                    vflip: true,
                    palbank,
                });
            }
            let hv_flipped = h_flipped.vflip();
            if let Some(tile_num) = self.patterns.iter().position(|img| *img == hv_flipped) {
                return Some(SbEntry {
                    tile_num,
                    hflip: true,
                    vflip: true,
                    palbank,
                });
            }
        }
        None
    }

    pub fn try_onboard_pattern(&mut self, new_img: Pattern, palbank: PalBankId) -> Option<SbEntry> {
        if let Some(sbe) = self.find_existing(&new_img, palbank) {
            Some(sbe)
        } else {
            self.try_onboard_without_reduction(new_img, palbank)
        }
    }

    pub fn try_onboard_without_reduction(
        &mut self,
        new_img: Pattern,
        palbank: PalBankId,
    ) -> Option<SbEntry> {
        if self.patterns.len() < self.max_patterns {
            let tile_num = self.patterns.len();
            self.patterns.push(new_img);
            Some(SbEntry {
                tile_num,
                hflip: false,
                vflip: false,
                palbank,
            })
        } else {
            None
        }
    }
}
