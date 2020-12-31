#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

/// # VRAM division by charblock:
///  0. level charblock
///  1. extra charblock (windmill)
///  2. hud charblock
///  3. screenblocks (add 24 to all of these):
///     0. and 1: level geometry (256x512 or 512x256 for scrolling)
///     2. and 3: foreground (as above)
///     4. and 5: skybox, or affine 512x512 for windmill
///     6. hud
///     7. ???
///  4. and 5. obj (subdivisions TBD)
///

pub mod palette;

use core::mem::{size_of, transmute};

use gba::vram::{Tile4bpp, Tile8bpp};
use gba::vram::{CHAR_BASE_BLOCKS, SCREEN_BASE_BLOCKS};
use gba::Color;

use gba::io::display::{
    DisplayControlSetting, DisplayStatusSetting, MosaicSetting, DISPCNT, DISPSTAT, MOSAIC,
    VBLANK_SCANLINE, VCOUNT,
};
use gba::io::window::{InsideWindowSetting, WININ};
use gba::palram::{PALRAM_BG, PALRAM_OBJ};

use flowergal_proj_config::resources::*;

use crate::timers::GbaTimer;
use crate::render::palette::{NO_EFFECT, VCOUNT_SEQUENCE, VCOUNT_SEQUENCE_LEN, NO_COLORS, TEXTBOX_VCOUNTS};
use core::ops::Range;

pub struct GbaRenderer {
    palette_normal_rom: &'static [Color],
    /// used for applying overlay/hardlight gradients every so many scanlines
    // TODO: compute obj blends
    palette_effect_rom: &'static [PaletteData],
    /// applied during hblank at vcount=113 and reverted at vcount=153.
    /// should be computed based on palette_normal or `palette_effect[133 / PAL_FX_SCANLINES]`
    palette_textbox_rom: &'static [Color],
    /// chosen during vcount irq, while waiting for timer1 irq.
    next_copy_pal: &'static [Color],
    /// workaround for next_copy_pal being insufficient alone for textbox_end when blend enabled
    next_copy_extra_norm_range: Range<usize>,
    dispstat: DisplayStatusSetting,
    showing_textbox: bool,
    showing_effect: bool,
    frame_counter: u32,
    vcount_index: usize,
    perf_log: [u32; VCOUNT_SEQUENCE_LEN],
    // shadow_oam: ShadowOam,
}

const fn palram_bg_slice() -> &'static mut [Color] {
    unsafe {
        &mut *core::ptr::slice_from_raw_parts_mut(
            PALRAM_BG.index_unchecked(0).to_usize() as *mut Color,
            PALRAM_BG.len()
        )
    }
}

const fn palram_obj_slice() -> &'static mut [Color] {
    unsafe {
        &mut *core::ptr::slice_from_raw_parts_mut(
            PALRAM_OBJ.index_unchecked(0).to_usize() as *mut Color,
            PALRAM_OBJ.len()
        )
    }
}

impl GbaRenderer {
    pub const fn new() -> Self {
        GbaRenderer {
            palette_normal_rom: NO_COLORS,
            palette_effect_rom: NO_EFFECT,
            palette_textbox_rom: NO_COLORS,
            next_copy_pal: NO_COLORS,
            next_copy_extra_norm_range: 0..0,
            dispstat: DisplayStatusSetting::new()
                .with_vblank_irq_enable(true)
                .with_vcounter_irq_enable(true),
            showing_textbox: false,
            showing_effect: false,
            frame_counter: 0,
            vcount_index: 0,
            perf_log: [0; VCOUNT_SEQUENCE_LEN],
        }
    }

    pub fn initialize(&mut self) {
    }

    pub fn vblank(&mut self) {
        self.frame_counter += 1;
    }

    pub fn even_odd_frame(&self) -> bool {
        self.frame_counter & 1 != 0
    }

    pub fn frame_counter(&self) -> u32 {
        self.frame_counter
    }

    #[link_section = ".iwram"]
    pub fn vcounter(&mut self) {
        // TODO: hit every other vcount and only set up timer1 if we're supposed to do a thing?
        // fudging the numbers a bit on this 750, but i'm assuming there'll be ~50 cycles overhead
        GbaTimer::setup_timer1_irq(750);

        let mut vcount = VCOUNT.read();
        if vcount >= VBLANK_SCANLINE /*- BLEND_RESOLUTION*/ as u16 {
            vcount = 0;
        }

        self.next_copy_pal = if self.showing_effect && !self.palette_effect_rom.is_empty()
            && !(self.showing_textbox && (TEXTBOX_VCOUNTS[0]..=TEXTBOX_VCOUNTS[1]).contains(&vcount)) {
            self.palette_effect_rom[vcount as usize / BLEND_RESOLUTION].data()
        } else if self.showing_textbox && vcount == TEXTBOX_VCOUNTS[0] {
            self.palette_textbox_rom
        } else if self.showing_textbox && vcount == TEXTBOX_VCOUNTS[1] {
            if self.showing_effect && !self.palette_effect_rom.is_empty() {
                let pal = self.palette_effect_rom[TEXTBOX_VCOUNTS[1] as usize / BLEND_RESOLUTION].data();
                let next_line_index = (pal.len() & !15) + 16;
                if next_line_index < self.palette_normal_rom.len() {
                    self.next_copy_extra_norm_range = next_line_index..self.palette_normal_rom.len();
                }
                pal
            } else {
                self.palette_normal_rom
            }
        } else {
            NO_COLORS
        };

        // TODO: screen wiggly effects

        /*
        // avoid flickering of bottom (sometimes irq firing on adjacent scanlines)
        if !(self.showing_textbox && vcount > TEXTBOX_Y_START && vcount <= TEXTBOX_Y_END) {
            while !DISPSTAT.read().hblank_flag() {
                // busy wait until it's safe
            }
        }
         */
    }

    #[link_section = ".iwram"]
    pub fn timer1(&mut self) {
        let start = GbaTimer::get_ticks();

        let pal = self.next_copy_pal;
        palram_bg_slice()[0..pal.len()].copy_from_slice(pal);
        if !self.next_copy_extra_norm_range.is_empty() {
            let mut range = 0..0;
            core::mem::swap(&mut self.next_copy_extra_norm_range, &mut range);
            palram_bg_slice()[range.clone()].copy_from_slice(&self.palette_normal_rom[range]);
        }

        self.perf_log[self.vcount_index] = GbaTimer::get_ticks() - start;
        self.vcount_index += 1;
        let mut next_vcount = VCOUNT_SEQUENCE[self.vcount_index];
        if next_vcount == 0xFF {
            #[cfg(feature = "bench_video")]
            warn!("pal copy: {:?}", &self.perf_log[..self.vcount_index]);
            self.vcount_index = 0;
            next_vcount = VCOUNT_SEQUENCE[self.vcount_index];
        }
        DISPSTAT.write(self.dispstat.with_vcount_setting(next_vcount));
    }

    pub fn set_color_effect_shown(&mut self, showing_effect: bool) {
        self.showing_effect = showing_effect;
        if !showing_effect {
            let pal = self.palette_normal_rom;
            let effect_len = self.palette_effect_rom.first()
                .map(|x| x.0.len() * 2)
                .unwrap_or(0);
            unsafe {
                palram_bg_slice()[..effect_len].copy_from_slice(&pal[..effect_len])
            }
        }
    }

    pub fn set_textbox_shown(&mut self, show: bool) {
        self.showing_textbox = show;
        if !show {
            let pal = self.palette_normal_rom;
            let effect_len = self.palette_effect_rom.first()
                .map(|x| x.0.len() * 2)
                .unwrap_or(0);
            unsafe {
                palram_bg_slice()[effect_len..pal.len()]
                    .copy_from_slice(&pal[effect_len..pal.len()])
            }
        }
    }

    pub fn set_normal_colors_bg(&mut self, index: usize, colors: &[gba::Color]) {
        palram_bg_slice()[index..(index + colors.len())].copy_from_slice(colors);
    }

    pub fn load_world_palettes(&mut self, world_pal: &WorldPalettes) {
        self.palette_normal_rom = world_pal.normal_palette.data();
        self.palette_effect_rom = world_pal.blended_palettes;
        self.palette_textbox_rom = world_pal.textbox_blend_palette.data();
        self.showing_effect = !world_pal.blended_palettes.is_empty();
        self.set_normal_colors_bg(0, self.palette_normal_rom);
        palram_bg_slice()[self.palette_normal_rom.len()..240].fill(gba::Color(0));
    }

    pub fn load_bg_tiles<T: Copy>(&self, charblock: u16, tiles: &[T]) {
        assert!(charblock < 4);
        assert!(tiles.len() * size_of::<T>() <= 256 * 8 * 8);
        info!("load_bg_tiles {}", charblock);
        let dest_addr = CHAR_BASE_BLOCKS.index(charblock as usize).to_usize();
        unsafe {
            core::slice::from_raw_parts_mut(dest_addr as *mut T, tiles.len())
                .copy_from_slice(tiles);
        }
    }

    pub fn load_bg_tiles_lz77(&self, charblock: u16, data: &[u32]) {
        assert!(charblock < 4);
        assert!(data[0] >> 8 <= 256 * 8 * 8);
        let dest_addr = CHAR_BASE_BLOCKS.index(charblock as usize).to_usize();
        unsafe {
            gba::bios::lz77_uncomp_16bit(data.as_ptr(), dest_addr as *mut u16);
        }
    }

    pub fn set_mosaic_amount(&self, mosaic_amount: u16) {
        MOSAIC.write(
            MosaicSetting::new()
                .with_bg_horizontal_inc(mosaic_amount)
                .with_bg_vertical_inc(mosaic_amount),
        );
    }
}

// in case we dynamic-draw tile pixels at runtime to change one
pub fn set_bg_tile_4bpp(charblock: u16, index: usize, tile: Tile4bpp) {
    assert!(charblock < 4);
    assert!(index < 512);
    unsafe {
        CHAR_BASE_BLOCKS
            .index(charblock as usize)
            .cast::<Tile4bpp>()
            .offset(index as isize)
            .write(tile)
    }
}
