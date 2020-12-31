/* https://deku.gbadev.org/program/sound1.html#buffer
      REG_TM0D                    frequency
         |                            |
         v                            v
Timer = 62610 = 65536 - (16777216 /  5734), buf = 96
Timer = 63408 = 65536 - (16777216 /  7884), buf = 132 <- the only buf that's not x8
Timer = 63940 = 65536 - (16777216 / 10512), buf = 176
Timer = 64282 = 65536 - (16777216 / 13379), buf = 224
Timer = 64612 = 65536 - (16777216 / 18157), buf = 304
Timer = 64738 = 65536 - (16777216 / 21024), buf = 352
Timer = 64909 = 65536 - (16777216 / 26758), buf = 448
Timer = 65004 = 65536 - (16777216 / 31536), buf = 528
Timer = 65073 = 65536 - (16777216 / 36314), buf = 608
Timer = 65118 = 65536 - (16777216 / 40137), buf = 672
Timer = 65137 = 65536 - (16777216 / 42048), buf = 704
 */

/// must be one of 5734, 7884, 10512, 13379, 18157, 21024, 26758, 31536, 36314, 40137, 42048
/// such that samples get divided evenly into vblank rate and cpu frequency
pub const SAMPLE_RATE: u16 = 31536;

pub const CPU_FREQ: u32 = 16777216;
pub const CYCLES_PER_FRAME: u32 = 280896; // = 16777216 Hz / 59.7275 FPS
/// this is the number of samples per frame.
pub const PLAYBUF_SIZE: usize =
    (((CYCLES_PER_FRAME as f32 * SAMPLE_RATE as f32) / CPU_FREQ as f32) + 0.5) as usize;
pub const SAMPLE_TIME: u32 = CYCLES_PER_FRAME / PLAYBUF_SIZE as u32; // = (16777216 / SAMPLE_RATE)
pub const TIMER_VALUE: u16 = (0x10000 - SAMPLE_TIME) as u16;

// must be a multiple of 8 for our handwritten ASM routines to work
const_assert_eq!(PLAYBUF_SIZE & 0x7, 0);

#[cfg_attr(not(target_arch = "arm"), derive(Clone))]
pub struct TrackList(pub &'static [MusicId]);

/// music index in the DEBUG jukebox
#[repr(usize)]
#[cfg_attr(not(target_arch = "arm"), derive(Debug))]
#[derive(Copy, Clone)]
pub enum MusicId {
    TomsDiner,
}

pub const SONG_FILES: &[&str] = &[
    "Tom's Diner [Long Version] DNA feat. Suzanne Vega (1990)-32ZTjFW2RYo.mkv",
];

/// sfx index in the jukebox
#[allow(non_camel_case_types)]
#[cfg_attr(not(target_arch = "arm"), derive(Debug))]
#[derive(Copy, Clone)]
pub enum SfxId {}

pub const SFX_FILES: &[&str] = &[];

#[cfg(not(target_arch = "arm"))]
mod impl_debug_for_build_const {
    use super::*;
    use core::fmt::{Debug, Formatter, Result};

    impl Debug for TrackList {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(f, "TrackList(&[")?;
            for id in self.0 {
                write!(f, "MusicId::{:?}, ", id)?;
            }
            write!(f, "])")
        }
    }
}
