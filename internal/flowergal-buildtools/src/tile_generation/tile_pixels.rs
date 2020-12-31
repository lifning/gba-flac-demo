use super::{PAL_LEN, TILE_H, TILE_W};

use std::collections::BTreeSet;

// TODO: compare tileimg redundancy on rgbpixels, not pal index images,
//  sort palette colors & *then* map colors
// map -[sliding window]-> TilePixels -[calculate most-redundant palettes]-> (Pattern, Palette)

#[derive(Eq, PartialEq, Clone)]
pub struct TilePixels {
    pub pixel_colors: Vec<gba::Color>,
    pub pal: BTreeSet<gba::Color>,
}

impl From<&[gba::Color]> for TilePixels {
    fn from(slice: &[gba::Color]) -> Self {
        TilePixels {
            pixel_colors: slice.to_vec(),
            pal: slice.iter().cloned().collect(),
        }
    }
}

pub struct TilePixelsBank {
    pub tiles: Vec<TilePixels>,
}
impl TilePixelsBank {
    pub fn new() -> Self {
        TilePixelsBank {
            tiles: vec![TilePixels::from(&[gba::Color(0); TILE_W * TILE_H][..])],
        }
    }

    pub fn try_onboard_tile(&mut self, pixel_colors: &[gba::Color]) -> usize {
        let new_pattern = TilePixels::from(pixel_colors);
        if let Some(tile_num) = self.tiles.iter().position(|img| *img == new_pattern) {
            tile_num
        } else {
            let tile_num = self.tiles.len();
            self.tiles.push(new_pattern);
            tile_num
        }
    }

    /// given a collection of 8x8 pixel ARGB1555 bitmap tiles, each with no more than 15 colors,
    /// find a clustering of tile groups, each sharing a 15-color palette,
    /// resulting in the fewest possible distinct palettes.
    ///
    /// Note: this problem is very similar to the NP-hard budgeted maximum coverage problem.
    /// See https://en.wikipedia.org/wiki/Maximum_coverage_problem for context.
    pub fn approximate_optimal_grouping(&self) -> Vec<(BTreeSet<gba::Color>, Vec<TilePixels>)> {
        let mut groupings: Vec<(BTreeSet<gba::Color>, Vec<TilePixels>)> = Vec::new();
        let mut remaining_tiles = self.tiles.clone();

        while !remaining_tiles.is_empty() {
            // find the tile whose colors superset those of the greatest number of other tiles
            let mut colors = remaining_tiles
                .iter()
                .max_by_key(|t| {
                    remaining_tiles
                        .iter()
                        .filter(|t2| t.pal.is_superset(&t2.pal))
                        .count()
                })
                .unwrap()
                .pal
                .clone();

            // it's free real estate!
            let mut tiles: Vec<_> = remaining_tiles
                .drain_filter(|t| t.pal.is_subset(&colors))
                .collect();

            // fill out the rest of colors until we hit PAL_LEN
            while colors.len() < PAL_LEN {
                // find tile with most colors common with this palette & won't break the max len,
                // with a tie breaker of how many other tiles it would let us superset (as above)
                let merge_colors_opt = remaining_tiles
                    .iter()
                    .filter(|t| t.pal.union(&colors).count() <= PAL_LEN)
                    .max_by_key(|t| {
                        (
                            t.pal.intersection(&colors).count(),
                            remaining_tiles
                                .iter()
                                .filter(|t2| {
                                    colors
                                        .union(&t.pal)
                                        .cloned()
                                        .collect::<BTreeSet<_>>()
                                        .is_superset(&t2.pal)
                                })
                                .count(),
                        )
                    })
                    .map(|t| &t.pal);
                if let Some(merge_colors) = merge_colors_opt {
                    colors.extend(merge_colors.iter());
                    tiles.extend(remaining_tiles.drain_filter(|t| t.pal.is_subset(&colors)));
                } else {
                    break;
                }
            }
            groupings.push((colors, tiles))
        }

        groupings
    }
}
