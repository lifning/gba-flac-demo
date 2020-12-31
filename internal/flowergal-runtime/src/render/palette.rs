/// # GBA display timing periods (courtesy TONC)
/// ```text
///       |<------- 240px ------->|<-68->|
///      _ ______________________________
///     ^ |>=== scanline == 1,232 cyc ==>|
///     | |                       |      |
///     | |>====== hdraw ========>|hblank|
/// 160px |    960 cycles (~4/px) |272cyc|
///     | |                       |      |
///     | |     *  vdraw          |      |
///     | | 197,120 cycles (= 1232 * 160)|
///     v_|_______________________| __ __|
///     ^ |  *  vblank                   |
/// 68px| |  83,776 cycles (= 1232 * 68) |
///     v_|______________________________|
/// ```
///
/// # Math time!
/// ```text
///       |<40>|<-- 160px -->|<40>|<-68->|
///      _ ______________________________
///     ^ |hud |gameplay area|hud |hblank|
///     | |<------200px----->|<----148px-@  {200*4 = 800cyc after VCount hits to start copy}
///     | @--->|             |    |      |  {148*4 = 596cyc to copy blends to PalRAM,
/// 160px |    |             |<----------@   (40*2)*4 = 320cyc of which have an addn'l waitstate}
///     | @---------------456px----------@
///     | @--->|   textbox   |    |      |  {456*4 = 1,824cyc to copy textbox to PalRAM,
///     | |    |=============|    |      |   (240+40*2)*4 = 1,280cyc of which have addn'l waitstate}
///     v_|____|_____________|____| __ __|
///     ^ |                              |  {copy y=0 blend into PalRAM and second palette to IWRAM
/// 68px| | 83,776cyc of vblank for game |   before end of vblank}
///     v_|______________________________|
/// ```
/// - on VCount interrupt (at x=0), set a timer IRQ to overflow in 800-n cycles, where n is the
///   number of cycles it takes to handle the VCount interrupt and set the timer registers
/// - on Timer interrupt (at x=200ish), start copying blend colors from ROM to PalRAM.
///   - 320c of 5c copies (amortized 2c/word read from ROM + 2c/word write to PalRAM + 1 waitstate)
///     - 320/5 = 64 words = 128 colors (minus overhead < 8 palette lines)
///   - 276c of 4c copies (amortized 2c/word read from ROM + 2c/word write to PalRAM)
///     - 276/4 = 69 words = 138 colors (minus overhead < 8.5 palette lines)

use flowergal_proj_config::resources::{PaletteData, BLEND_ENTRIES, TEXTBOX_Y_START, TEXTBOX_Y_END, BLEND_RESOLUTION};
use gba::io::display::VBLANK_SCANLINE;

const ZERO: u32 = 0;

pub(crate) const NO_EFFECT: &[PaletteData] = &[];
pub(crate) const NO_COLORS: &[gba::Color] = &[];

/// must be in sorted order
pub const TEXTBOX_VCOUNTS: [u16; 2] = [TEXTBOX_Y_START - 1, TEXTBOX_Y_END];

/// last vcount interrupt that fires on hardware
const VCOUNT_LAST: u16 = VBLANK_SCANLINE + 68 - 1; // 227

/// two passes of the screen in cycles, one with vcounts offset by half the blend resolution
/// for flickering between palette swap lines to smooth horizontal bands.
/// (playing on emulator? turn on "interframe blending")
pub(crate) const VCOUNT_SEQUENCE_LEN: usize = 2 * (BLEND_ENTRIES + TEXTBOX_VCOUNTS.len() + 1);
pub(crate) const VCOUNT_SEQUENCE: [u16; VCOUNT_SEQUENCE_LEN] = compute_vcount_sequence();
/// used in the above array to signify that we should rewind
const VCOUNT_INVALID: u16 = 0xFF;

#[allow(clippy::comparison_chain)]
const fn compute_vcount_sequence() -> [u16; VCOUNT_SEQUENCE_LEN] {
    let mut vcounts = [VCOUNT_INVALID; VCOUNT_SEQUENCE_LEN];
    let offsets: [u16; 2] = [BLEND_RESOLUTION as u16, BLEND_RESOLUTION as u16 / 2];
    let mut oi = 0;
    let mut vi = 0;

    while oi < offsets.len() {
        let mut line = offsets[oi];
        let mut ti = 0;
        while line < VBLANK_SCANLINE {
            if ti < TEXTBOX_VCOUNTS.len() {
                let y = TEXTBOX_VCOUNTS[ti];
                if line > y {
                    vcounts[vi] = y;
                    vi += 1;
                    ti += 1;
                    continue;
                } else if line == y {
                    ti += 1;
                }
            }
            vcounts[vi] = line;
            vi += 1;
            line += BLEND_RESOLUTION as u16;
        }
        vcounts[vi] = VCOUNT_LAST - 5;
        vi += 1;
        oi += 1;
    }

    vcounts
}
