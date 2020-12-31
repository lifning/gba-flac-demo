pub use gba::vram::affine::AffineScreenblockEntry;
pub use gba::vram::text::TextScreenblockEntry;
pub use gba::vram::{Tile4bpp, Tile8bpp};
pub use gba::Color;

use crate::sound_info::TrackList;
use crate::WorldId;

pub mod blend;

pub const ROOM_SIZE: (usize, usize) = (32, 32);

pub const TEXT_TOP_ROW: isize = 13;
pub const TEXT_BOTTOM_ROW: isize = TEXT_TOP_ROW + 4;

pub const TEXTBOX_Y_START: u16 = TEXT_TOP_ROW as u16 * 8 + 2;
pub const TEXTBOX_Y_END: u16 = TEXT_BOTTOM_ROW as u16 * 8 + 5;
pub const TEXTBOX_Y_MID_EFFECT_INDEX: usize = (TEXTBOX_Y_START + 20) as usize / BLEND_RESOLUTION;

// TODO: source from image file?
pub const TEXTBOX_R: u16 = 42;
pub const TEXTBOX_G: u16 = 42;
pub const TEXTBOX_B: u16 = 69;
pub const TEXTBOX_A: u16 = 141;

/// resolution of palette effects (not every scanline for memory and cpu's sake)
/// must be a power of 2 for ARMv4's sake (CPU long division during hblank is a bad plan)
pub const BLEND_RESOLUTION: usize = if cfg!(debug_assertions) { 16 } else { 4 };
/// note: this many KiB get used by the blend cache in EWRAM, adjust RESOLUTION accordingly...
pub const BLEND_ENTRIES: usize = 160 / BLEND_RESOLUTION;

#[repr(align(4))]
pub struct AlignWrapper<T>(pub T);

#[repr(transparent)]
#[cfg_attr(not(target_arch = "arm"), derive(Debug))]
pub struct RoomEntries4bpp(pub [[TextScreenblockEntry; ROOM_SIZE.0]; ROOM_SIZE.1]);

#[repr(transparent)]
#[cfg_attr(not(target_arch = "arm"), derive(Debug))]
pub struct RoomEntries8bpp(pub [[AffineScreenblockEntry; ROOM_SIZE.0/2]; ROOM_SIZE.1/2]);

#[repr(transparent)]
pub struct Metamap(pub &'static [&'static [u8]]);

pub enum TilePatterns {
    Text(&'static [Tile4bpp]),
    Affine(&'static [Tile8bpp]),
    TextLz77(&'static [u32]),
    AffineLz77(&'static [u32]),
}

pub enum RoomData {
    Text(&'static [RoomEntries4bpp]),
    Affine(&'static [RoomEntries8bpp]),
    TextLz77(&'static [&'static [u32]]),
}

#[cfg_attr(not(target_arch = "arm"), derive(Debug))]
pub struct Layer {
    pub map: RoomData,
    pub meta: Metamap,
}

// alignment woes...
#[derive(Clone)]
pub struct PaletteData(pub &'static [u32]);

impl PaletteData {
    pub const fn new(c: &'static [Color]) -> Self {
        let ptr = c.as_ptr();
        let len = (c.len() + 1) / 2;
        unsafe { PaletteData(&*core::ptr::slice_from_raw_parts(ptr as *const u32, len)) }
    }
    pub const fn data(&self) -> &'static [Color] {
        let ptr = self.0.as_ptr();
        let len = self.0.len() * 2;
        unsafe { &*core::ptr::slice_from_raw_parts(ptr as *const Color, len) }
    }
}

#[derive(Clone)]
pub struct WorldPalettes {
    pub normal_palette: PaletteData,
    pub blended_palettes: &'static [PaletteData],
    pub textbox_blend_palette: PaletteData,
}

pub enum ColorEffectType {
    None,
    Overlay,
    HardLight,
    Multiply,
}

#[repr(align(4))]
pub enum Sound {
    RawPcm8(&'static [u8]),
    Flac(&'static [u8]),
}

impl Sound {
    pub fn data_ptr(&self) -> *const u8 {
        match self {
            Sound::RawPcm8(x) | Sound::Flac(x) => x.as_ptr(),
        }
    }
}

#[cfg_attr(not(target_arch = "arm"), derive(Debug))]
pub struct WorldData {
    pub id: WorldId,
    pub name: &'static str,
    pub pal: WorldPalettes,
    pub img: TilePatterns,
    pub img_special: TilePatterns,
    pub bg_layer: Option<Layer>,
    pub fg_layer: Option<Layer>,
    pub skybox_layer: Option<Layer>,
    // TODO: pub anim_tiles: { Grid }
    // TODO: this'll have to be referential rather than a copy of the data.
    //  (some songs like Mitra's theme are used in multiple places, plus DEBUG has a soundtest)
    pub music: TrackList,
}

/// The impl Debug from the derive macro doesn't put amps in front of references,
/// or exist for statically sized arrays larger than 32.
#[cfg(not(target_arch = "arm"))]
mod debug_impls_for_build_const {
    use super::*;
    use std::fmt::Formatter;

    impl core::fmt::Debug for PaletteData {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(f, "PaletteData(&{:?})", self.0)
        }
    }

    impl core::fmt::Debug for Metamap {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            writeln!(f, "Metamap(&[")?;
            for row in self.0 {
                writeln!(f, "    &{:?},", row)?;
            }
            write!(f, "])")
        }
    }

    impl core::fmt::Debug for TilePatterns {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                TilePatterns::Text(data) => write!(f, "TilePatterns::Text(&{:?})", data),
                TilePatterns::Affine(data) => write!(f, "TilePatterns::Affine(&{:?})", data),
                TilePatterns::TextLz77(data) => write!(f, "TilePatterns::TextLz77(&{:?})", data),
                TilePatterns::AffineLz77(data) => {
                    write!(f, "TilePatterns::AffineLz77(&{:?})", data)
                }
            }
        }
    }

    impl core::fmt::Debug for RoomData {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                RoomData::Text(data) => write!(f, "RoomData::Text(&{:?})", data),
                RoomData::Affine(data) => write!(f, "RoomData::Affine(&{:?})", data),
                RoomData::TextLz77(data) => {
                    writeln!(f, "RoomData::TextLz77(&[")?;
                    for d in *data {
                        writeln!(f, "    &{:?},", *d)?;
                    }
                    write!(f, "])")
                }
            }
        }
    }

    impl core::fmt::Debug for WorldPalettes {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            writeln!(f, "WorldPalettes {{")?;
            writeln!(f, "    normal_palette: {:?},", self.normal_palette)?;
            writeln!(f, "    blended_palettes: &[")?;
            for row in self.blended_palettes.iter() {
                writeln!(f, "        {:?},", row)?;
            }
            writeln!(f, "    ],")?;
            writeln!(
                f,
                "    textbox_blend_palette: {:?},",
                self.textbox_blend_palette
            )?;
            write!(f, "}}")
        }
    }

    impl core::fmt::Debug for Sound {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            match self {
                Sound::RawPcm8(data) => write!(f, "Sound::RawPcm8(&{:?})", data),
                Sound::Flac(data) => write!(f, "Sound::Flac(&{:?})", data),
            }
        }
    }
}
