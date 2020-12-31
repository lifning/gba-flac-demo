// Copyright (C) 2021 lifning, licensed under the GNU Affero General Public License version 3.

use std::error::Error;

use sdl2::pixels::Color as sdl_Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::surface::Surface;

use crate::tile_generation::{TILE_H, TILE_W};

pub fn pixels_of_rect_32bit(
    surf: &Surface,
    rect: Rect,
    solid_bg: Option<sdl_Color>,
) -> Result<Vec<sdl_Color>, Box<dyn Error>> {
    let (width, height) = rect.size();
    let dst_rect = Rect::new(0, 0, width, height);

    let mut dst = Surface::new(width, height, PixelFormatEnum::RGBA8888)?;
    assert_eq!(dst.pixel_format_enum(), PixelFormatEnum::RGBA8888);
    if let Some(c) = solid_bg {
        dst.fill_rect(dst_rect, c)?;
    }
    surf.blit(rect, &mut dst, dst_rect)?;
    let pitch = dst.pitch();
    let pix_fmt = dst.pixel_format();

    let color_vec = dst.with_lock(|pixels_raw| {
        let mut pixels_vec = Vec::new();
        for y in 0..height {
            let subslice = unsafe {
                core::slice::from_raw_parts(
                    (pixels_raw.as_ptr().offset((pitch * y) as isize)) as *const u32,
                    width as usize,
                )
            };
            pixels_vec.extend_from_slice(subslice);
        }
        pixels_vec
            .into_iter()
            .map(|c| sdl_Color::from_u32(&pix_fmt, c))
            .collect()
    });
    Ok(color_vec)
}

pub fn pixels_of_rect_16bit(surf: &Surface, rect: Rect) -> Result<Vec<gba::Color>, Box<dyn Error>> {
    Ok(sdl_to_gba_colors(pixels_of_rect_32bit(surf, rect, None)?))
}

pub fn sdl_to_gba_colors(sdl_p: impl IntoIterator<Item = sdl_Color>) -> Vec<gba::Color> {
    sdl_p
        .into_iter()
        .map(|c| {
            // GBA is XBGR1555
            let r = c.r as u16 >> 3;
            let g = c.g as u16 >> 3;
            let b = c.b as u16 >> 3;
            // GBA ignores highest bit, but we use it during our own processing
            let a = ((c.a > 127) as u16) << 15;
            gba::Color(gba::Color::from_rgb(r, g, b).0 | a)
        })
        .collect()
}

pub fn pixels_of_tile(
    surf: &Surface,
    tx: usize,
    ty: usize,
) -> Result<Vec<gba::Color>, Box<dyn Error>> {
    let src_rect = Rect::new(
        (tx * TILE_W) as i32,
        (ty * TILE_H) as i32,
        TILE_W as u32,
        TILE_H as u32,
    );
    let pixels = pixels_of_rect_16bit(&surf, src_rect)?;
    assert_eq!(pixels.len(), TILE_W * TILE_H);
    Ok(pixels)
}
