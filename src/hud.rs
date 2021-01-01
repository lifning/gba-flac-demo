use gba::io::background::BackgroundControlSetting;
use gba::io::background::{BGSize, BG0CNT};
use gba::vram::text::TextScreenblockEntry;
use gba::vram::SCREEN_BASE_BLOCKS;

use voladdress::VolAddress;

use flowergal_proj_assets::{UI_IMG, UI_PAL};
use flowergal_runtime::Driver;
use flowergal_proj_config::resources::{TEXT_TOP_ROW, TEXT_BOTTOM_ROW};

type TSE = TextScreenblockEntry;

const HUD_SCREENBLOCK_ID: u16 = 30;
const HUD_SCREENBLOCK: VolAddress<TextScreenblockEntry> = unsafe {
    SCREEN_BASE_BLOCKS
        .index_unchecked(HUD_SCREENBLOCK_ID as usize)
        .cast()
};
pub const HUD_CHARBLOCK_ID: u16 = 2;
pub const HUD_PALETTE: u16 = 15;

pub const HUD_LEFT_COL: isize = 4;
pub const HUD_RIGHT_COL: isize = HUD_LEFT_COL + 21;

pub enum Button {
    A,
    B,
}

pub struct Hud {}

impl Hud {
    pub fn new() -> Self {
        let bg_settings = BackgroundControlSetting::new()
            .with_screen_base_block(HUD_SCREENBLOCK_ID)
            .with_char_base_block(HUD_CHARBLOCK_ID)
            .with_mosaic(true)
            .with_bg_priority(0)
            .with_size(BGSize::Zero);

        BG0CNT.write(bg_settings);

        let renderer = unsafe { Driver::instance_mut() }.video();
        renderer.set_normal_colors_bg(240, &UI_PAL);
        renderer.load_bg_tiles(HUD_CHARBLOCK_ID, &UI_IMG);

        Hud {}
    }

    fn odd_frame(&self) -> bool {
        unsafe { Driver::instance_mut().video() }.frame_counter & 1 != 0
    }

    fn write_entry(&self, row: isize, col: isize, entry: TSE) {
        unsafe { HUD_SCREENBLOCK.offset(row * 32 + col) }
            .write(TextScreenblockEntry(entry.with_palbank(HUD_PALETTE).0 + 1))
    }

    fn write_entry_alternating(&self, row: isize, col: isize, entry: TSE) {
        // hack
        let alternation = if col > 15 {
            !self.odd_frame()
        } else {
            self.odd_frame()
        } as u16;
        self.write_entry(row, col, TextScreenblockEntry(entry.0 + alternation))
    }

    pub fn draw_background(&self) {
        for row in 0..20 {
            for col in 0..30 {
                let entry = match (col, row) {
                    (0..=4, _) | (25..=29, _) => tiles::BG,
                    _ => tiles::NONE,
                };
                self.write_entry(row, col, entry);
            }
        }
    }

    pub fn clear_text_area(&self) {
        let left = HUD_LEFT_COL + 1;
        let right = left + 20;
        for row in TEXT_TOP_ROW..=TEXT_BOTTOM_ROW {
            for col in left..right {
                self.write_entry(row, col, tiles::NONE);
            }
        }
    }

    pub fn draw_borders(&mut self, with_textbox: bool) {
        for row in 0..20 {
            self.write_entry_alternating(row, HUD_LEFT_COL, tiles::BG_LEFT_EDGE_1);
            self.write_entry_alternating(row, HUD_RIGHT_COL, tiles::BG_RIGHT_EDGE_1);
        }
        if with_textbox {
            self.write_entry_alternating(TEXT_TOP_ROW, HUD_LEFT_COL, tiles::TEXTBOX_TL_1);
            for i in (HUD_LEFT_COL+1)..=(HUD_RIGHT_COL-1) {
                self.write_entry(TEXT_TOP_ROW, i, tiles::TEXTBOX_T);
            }
            self.write_entry_alternating(TEXT_TOP_ROW, HUD_RIGHT_COL, tiles::TEXTBOX_TR_1);
            for i in (TEXT_TOP_ROW+1)..=(TEXT_BOTTOM_ROW-1) {
                self.write_entry_alternating(i, HUD_LEFT_COL, tiles::TEXTBOX_L_1);
                self.write_entry_alternating(i, HUD_RIGHT_COL, tiles::TEXTBOX_R_1);
            }
            self.write_entry_alternating(TEXT_BOTTOM_ROW, HUD_LEFT_COL, tiles::TEXTBOX_BL_1);
            for i in (HUD_LEFT_COL+1)..=(HUD_RIGHT_COL-1) {
                self.write_entry(TEXT_BOTTOM_ROW, i, tiles::TEXTBOX_B);
            }
            self.write_entry_alternating(TEXT_BOTTOM_ROW, HUD_RIGHT_COL, tiles::TEXTBOX_BR_1);
        }
    }

    pub fn draw_text(&self, string: &str) {
        let left = HUD_LEFT_COL + 1;
        let right = left + 20;
        let mut row = TEXT_TOP_ROW + 1;
        let mut col = left;
        for c in string.chars() {
            if c == '\n' {
                row += 1;
                col = left;
            } else {
                if col >= right {
                    row += 1;
                    col = left;
                }
                self.write_entry(row, col, TSE::from_tile_id(c as u16));
                col += 1;
            }
        }
    }
}

mod tiles {
    use super::*;
    pub const NONE: TSE = TSE::from_tile_id(0);
    pub const BG: TSE = TSE::from_tile_id(1);
    pub const BG_LEFT_EDGE_1: TSE = TSE::from_tile_id(2);
    // pub const BG_LEFT_EDGE_2: TSE = TSE::from_tile_id(3);
    pub const BG_RIGHT_EDGE_1: TSE = TSE::from_tile_id(2).with_hflip(true);
    // pub const BG_RIGHT_EDGE_2: TSE = TSE::from_tile_id(3).with_hflip(true);

    pub const TEXTBOX_TL_1: TSE = TSE::from_tile_id(4);
    // pub const TEXTBOX_TL_2: TSE = TSE::from_tile_id(5);
    pub const TEXTBOX_TR_1: TSE = TSE::from_tile_id(4).with_hflip(true);
    // pub const TEXTBOX_TR_2: TSE = TSE::from_tile_id(5).with_hflip(true);
    pub const TEXTBOX_L_1: TSE = TSE::from_tile_id(6);
    // pub const TEXTBOX_L_2: TSE = TSE::from_tile_id(7);
    pub const TEXTBOX_R_1: TSE = TSE::from_tile_id(6).with_hflip(true);
    // pub const TEXTBOX_R_2: TSE = TSE::from_tile_id(7).with_hflip(true);
    pub const TEXTBOX_BL_1: TSE = TSE::from_tile_id(8);
    // pub const TEXTBOX_BL_2: TSE = TSE::from_tile_id(9);
    pub const TEXTBOX_BR_1: TSE = TSE::from_tile_id(8).with_hflip(true);
    // pub const TEXTBOX_BR_2: TSE = TSE::from_tile_id(9).with_hflip(true);
    pub const TEXTBOX_T: TSE = TSE::from_tile_id(10);
    pub const TEXTBOX_B: TSE = TSE::from_tile_id(10).with_vflip(true);
}
