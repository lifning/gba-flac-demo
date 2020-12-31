use std::error::Error;

use gen_iter::GenIter;

use sdl2::surface::Surface;

mod image_bank;
mod map;
mod palette;
mod pattern;
mod tile_pixels;

pub mod sdl_support;

pub use image_bank::*;
pub use map::*;
pub use palette::*;
pub use pattern::*;

const TILE_W: usize = 8;
const TILE_H: usize = 8;

// generalize so we can do 24x24, etc.?
fn traverse_16_grid_as_8(
    width_16tiles: usize,
    height_16tiles: usize,
) -> impl Iterator<Item = (usize, usize)> {
    GenIter(move || {
        for y in 0..height_16tiles / 2 {
            for x in 0..width_16tiles / 2 {
                for suby in &[0, 1] {
                    for subx in &[0, 1] {
                        yield (x * 2 + subx, y * 2 + suby);
                    }
                }
            }
        }
    })
}

/*
pub fn process_maps<'a>(
    map_surfs: impl IntoIterator<Item = Surface<'a>>,
    max_colors: usize,
    is_4bpp: bool,
) -> Result<(Vec<RoomBank>, ImageBank), Box<dyn Error>> {
    let mut bank = ImageBank::new(max_colors, is_4bpp);

    let mut metamaps = Vec::new();

    for surf in map_surfs.into_iter() {
        metamaps.push(bank.process_world_map(&surf, false)?);
    }

    Ok((metamaps, bank))
}
*/

pub fn process_basic_tilesets<'a>(
    surfaces: impl IntoIterator<Item = Surface<'a>>,
    max_colors: usize,
) -> Result<(Vec<Grid>, ImageBank), Box<dyn Error>> {
    let mut bank = ImageBank::new(max_colors, true);

    let mut grids = Vec::new();

    for surf in surfaces.into_iter() {
        grids.push(bank.process_image_region(&surf, None, false, false, false)?);
    }

    Ok((grids, bank))
}
