// Copyright (C) 2021 lifning, licensed under the GNU Affero General Public License version 3.

use std::error::Error;

use sdl2::rect::Rect;
use sdl2::surface::Surface;

use flowergal_proj_config::resources::{PaletteData, TilePatterns};

use crate::tile_generation::tile_pixels::TilePixelsBank;
use crate::tile_generation::{sdl_support, traverse_16_grid_as_8, Grid, PaletteBank, Pattern, PatternBank, RoomBank, SbEntry, PAL_LEN, TILE_H, TILE_W, PalBankId};

pub struct ImageBank {
    // charblock 0
    pub pattern_bank: PatternBank,
    // charblock 1
    pub pattern_bank_special: PatternBank,
    pub palette_bank: PaletteBank,
    pub tile_pixels_bank: TilePixelsBank,
    pub tile_pixels_bank_special: TilePixelsBank,
    pub special_is_4bpp: bool,
}

impl ImageBank {
    pub fn new(max_colors: usize, special_is_4bpp: bool) -> Self {
        assert!(max_colors <= 256);
        ImageBank {
            pattern_bank: PatternBank::new(true),
            pattern_bank_special: PatternBank::new(special_is_4bpp),
            palette_bank: PaletteBank::new(max_colors),
            tile_pixels_bank: TilePixelsBank::new(),
            tile_pixels_bank_special: TilePixelsBank::new(),
            special_is_4bpp,
        }
    }

    pub fn process_world_map(
        &mut self,
        surf: &Surface,
        special: bool,
        tile8_is_blended: impl Fn(usize, usize) -> bool,
        (room_width, room_height): (usize, usize)
    ) -> Result<RoomBank, Box<dyn Error>> {
        let is_4bpp = !special || self.special_is_4bpp;

        // optimize palette color packing (feeding them to pattern/palette banks in the right order)
        if is_4bpp {
            let mut tile_pixels_bank_plain = TilePixelsBank::new();
            let mut tile_pixels_bank_blend = TilePixelsBank::new();
            let grid_width = surf.width() as usize / TILE_W;
            let grid_height = surf.height() as usize / TILE_H;
            for ty in 0..grid_height {
                for tx in 0..grid_width {
                    let pixel_colors = sdl_support::pixels_of_tile(&surf, tx, ty)?;
                    if tile8_is_blended(tx, ty) {
                        tile_pixels_bank_blend.try_onboard_tile(&pixel_colors);
                    } else {
                        tile_pixels_bank_plain.try_onboard_tile(&pixel_colors);
                    }
                }
            }
            for (_colors, tiles) in tile_pixels_bank_blend.approximate_optimal_grouping() {
                for tile in tiles.iter() {
                    self.try_onboard_tile_colors(&tile.pixel_colors, true, special, true)
                        .ok_or_else(|| self.error(0, 0xffff).unwrap_err())?;
                }
            }
            for (_colors, tiles) in tile_pixels_bank_plain.approximate_optimal_grouping() {
                /* TODO: so the palette is in a sorted order?  currently breaks.
                self.palette_bank
                    .try_onboard_colors(&colors.into_iter().collect::<Vec<_>>())
                    .ok_or("Couldn't onboard palette colors during palette optimization pass")?;*/
                for tile in tiles.iter() {
                    self.try_onboard_tile_colors(&tile.pixel_colors, true, special, false)
                        .ok_or_else(|| self.error(0xffff, 0).unwrap_err())?;
                }
            }
        }

        let mut metamap = RoomBank::new(surf.size(), (room_width, room_height), is_4bpp);
        for ry in 0..(metamap.map_height / room_height) {
            for rx in 0..(metamap.map_width / room_width) {
                for (x, y) in traverse_16_grid_as_8(room_width, room_height) {
                    let tx = (rx * room_width) + x;
                    let ty = (ry * room_height) + y;
                    let pixel_colors = sdl_support::pixels_of_tile(&surf, tx, ty)?;

                    let blend = tile8_is_blended(tx, ty);
                    if let Some(mapsel) = self.try_onboard_tile_colors(&pixel_colors, true, special, blend)
                    {
                        metamap.set_sbe(tx, ty, mapsel);
                    } else {
                        self.error(tx, ty)?;
                    }
                }
            }
        }

        Ok(metamap)
    }

    pub fn process_image_region(
        &mut self,
        surf: &Surface,
        rect: Option<Rect>,
        deduplicate: bool,
        special: bool,
        blend: bool,
    ) -> Result<Grid, Box<dyn Error>> {
        let rect = rect.unwrap_or_else(|| surf.rect());
        let grid_size = (
            rect.width() as usize / TILE_W,
            rect.height() as usize / TILE_H,
        );
        let ofsx = rect.x() as usize / TILE_W;
        let ofsy = rect.y() as usize / TILE_H;
        let is_4bpp = !special || self.special_is_4bpp;
        let mut grid = Grid::new(grid_size, is_4bpp);

        for ty in 0..grid.grid_height {
            for tx in 0..grid.grid_width {
                let pixel_colors = sdl_support::pixels_of_tile(&surf, tx + ofsx, ty + ofsy)?;
                if let Some(mapsel) =
                    self.try_onboard_tile_colors(&pixel_colors, deduplicate, special, blend)
                {
                    grid.set_sbe(tx, ty, mapsel);
                } else {
                    self.error(tx, ty)?;
                }
            }
        }

        Ok(grid)
    }

    pub fn try_onboard_tile_colors(
        &mut self,
        pixel_colors: &[gba::Color],
        deduplicate: bool,
        special: bool,
        blend: bool,
    ) -> Option<SbEntry> {
        if let Some((mut pattern, mut palbank)) = self.palette_bank.try_onboard_colors(pixel_colors, blend)
        {
            let pattern_bank;
            if special {
                if !self.special_is_4bpp {
                    // FIXME: how to *actually* handle this weird case with PalBankId?
                    //  for now assuming that it's blended; i think the only 8bpp specials are.
                    let palbank_index = if let PalBankId::Blended(x) = palbank {
                        x
                    } else {
                        unimplemented!("non-blended 8bpp tiles??")
                    };
                    for x in pattern.0.iter_mut() {
                        *x += palbank_index * PAL_LEN;
                    }
                    palbank = PalBankId::Blended(0);
                }
                pattern_bank = &mut self.pattern_bank_special;
            } else {
                pattern_bank = &mut self.pattern_bank;
            };

            let option_sb_entry = if deduplicate {
                pattern_bank.try_onboard_pattern(pattern, palbank)
            } else {
                pattern_bank.try_onboard_without_reduction(pattern, palbank)
            };
            if let Some(sb_entry) = option_sb_entry {
                return Some(sb_entry);
            }
        }
        None
    }

    fn collect_patterns<'a, T: From<&'a Pattern>>(pattern_bank: &'a PatternBank) -> &'static [T] {
        let mut vec = Vec::with_capacity(pattern_bank.patterns.len());
        for pattern in &pattern_bank.patterns {
            vec.push(pattern.into())
        }
        Box::leak(vec.into_boxed_slice())
    }

    pub fn gba_patterns(&self, special: bool) -> TilePatterns {
        if special {
            if self.special_is_4bpp {
                TilePatterns::Text(Self::collect_patterns(&self.pattern_bank_special))
            } else {
                TilePatterns::Affine(Self::collect_patterns(&self.pattern_bank_special))
            }
        } else {
            TilePatterns::Text(Self::collect_patterns(&self.pattern_bank))
        }
    }

    pub fn gba_palette_full(&self) -> PaletteData {
        let mut vec = Vec::new();

        for pal in self.palette_bank.palettes_blend.iter()
            .chain(self.palette_bank.palettes_plain.iter())
        {
            for c in &pal.colors {
                vec.push(*c);
            }
            // each line must be the same length in the output, or else subsequent lines will be wrong
            // TODO: consider 2D array?
            for _ in pal.colors.len()..16 {
                vec.push(gba::Color(0));
            }
        }

        while *vec.last().unwrap_or(&gba::Color(1)) == gba::Color(0) {
            vec.pop();
        }

        PaletteData::new(Box::leak(vec.into_boxed_slice()))
    }

    pub fn blend_palette_size(&self) -> usize {
        if self.palette_bank.palettes_blend.is_empty() {
            0
        } else {
            let padded = self.palette_bank.palettes_blend.len() * 16;
            let last_len = self.palette_bank.palettes_blend.last().unwrap().colors.len();
            padded - (16 - last_len)
        }
    }

    fn error(&self, tx: usize, ty: usize) -> Result<(), Box<dyn Error>> {
        for (i, pal) in self.palette_bank.palettes_plain.iter().enumerate() {
            eprintln!(
                "palette {} length: {}/{}",
                i,
                pal.colors.len(),
                pal.max_colors
            );
        }
        Err(format!(
            "Could not add tile at ({}, {}); palettes {}+{}/{}, images {}/{}, special {}/{}",
            tx * TILE_W,
            ty * TILE_H,
            self.palette_bank.palettes_plain.len(),
            self.palette_bank.palettes_blend.len(),
            self.palette_bank.max_palettes,
            self.pattern_bank.patterns.len(),
            self.pattern_bank.max_patterns,
            self.pattern_bank_special.patterns.len(),
            self.pattern_bank_special.max_patterns,
        ).into())
    }
}
