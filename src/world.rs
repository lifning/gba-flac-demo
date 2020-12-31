use core::mem::size_of;

use gba::bios::BgAffineSetParams;
use gba::io::background::{
    BGSize, BackgroundControlSetting, BG1CNT, BG1HOFS, BG1VOFS, BG2CNT, BG2HOFS, BG2VOFS, BG3CNT,
    BG3HOFS, BG3VOFS,
};
use gba::io::display::{DisplayControlSetting, DisplayMode, DISPCNT};
use gba::vram::SCREEN_BASE_BLOCKS;

use voladdress::VolAddress;

use flowergal_proj_config::resources::{Layer, RoomData, TextScreenblockEntry, TilePatterns, WorldData};
use flowergal_proj_config::WorldId;

use flowergal_runtime::{Driver, MemoryOps, CoreLib};

use flowergal_proj_assets::MUSIC_DATA;
use gba::io::color_blend::{
    AlphaBlendingSetting, ColorEffectSetting, ColorSpecialEffect, BLDALPHA, BLDCNT,
};

const WORLD_CHARBLOCK_ID: u16 = 0;
const WORLD_CHARBLOCK_SPECIAL_ID: u16 = 1;

const WORLD_SCREENBLOCK_BG_ID: u16 = 24;
const WORLD_SCREENBLOCK_FG_ID: u16 = 26;
const WORLD_SCREENBLOCK_SKYBOX_ID: u16 = 28;

const WORLD_SCREENBLOCK_BG: VolAddress<TextScreenblockEntry> = unsafe {
    SCREEN_BASE_BLOCKS
        .index_unchecked(WORLD_SCREENBLOCK_BG_ID as usize)
        .cast()
};
const WORLD_SCREENBLOCK_FG: VolAddress<TextScreenblockEntry> = unsafe {
    SCREEN_BASE_BLOCKS
        .index_unchecked(WORLD_SCREENBLOCK_FG_ID as usize)
        .cast()
};
const WORLD_SCREENBLOCK_SKYBOX: VolAddress<TextScreenblockEntry> = unsafe {
    SCREEN_BASE_BLOCKS
        .index_unchecked(WORLD_SCREENBLOCK_SKYBOX_ID as usize)
        .cast()
};

const TEXT_SCREENBLOCK_TILES: usize = 32;
const ROOM_TILES: usize = 32;

const BG_HOFS_BASE: u16 = 0;
const BG_VOFS_BASE: u16 = 0;

#[repr(usize)]
#[derive(Clone, Copy)]
enum ScreenblockAddress {
    Background = WORLD_SCREENBLOCK_BG.to_usize(),
    Foreground = WORLD_SCREENBLOCK_FG.to_usize(),
    Skybox = WORLD_SCREENBLOCK_SKYBOX.to_usize(),
}

impl ScreenblockAddress {
    fn blocks_as_mut_slice<T>(self, n: usize) -> &'static mut [T] {
        unsafe {
            core::slice::from_raw_parts_mut(self as usize as *mut T, (0x800 * n) / size_of::<T>())
        }
    }
}

enum ScreenblockCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

enum SkyboxBehavior {
    None,
    Windmill,
}

impl ScreenblockCorner {
    pub fn text_offset(&self) -> usize {
        let tile_offset_within = TEXT_SCREENBLOCK_TILES - ROOM_TILES;
        let second_block = 0x800;
        match self {
            ScreenblockCorner::TopLeft => {
                (TEXT_SCREENBLOCK_TILES + 1)
                    * tile_offset_within
                    * size_of::<TextScreenblockEntry>()
            }
            ScreenblockCorner::TopRight => {
                second_block
                    + TEXT_SCREENBLOCK_TILES
                        * tile_offset_within
                        * size_of::<TextScreenblockEntry>()
            }
            ScreenblockCorner::BottomLeft => {
                second_block + tile_offset_within * size_of::<TextScreenblockEntry>()
            }
            ScreenblockCorner::BottomRight => {
                panic!("text screenblocks in this game don't have a bottom-right!")
            }
        }
    }
}
impl From<(usize, usize)> for ScreenblockCorner {
    fn from(pair: (usize, usize)) -> Self {
        match pair {
            (0, 0) => ScreenblockCorner::TopLeft,
            (1, 0) => ScreenblockCorner::TopRight,
            (0, 1) => ScreenblockCorner::BottomLeft,
            (1, 1) => ScreenblockCorner::BottomRight,
            (x, y) => panic!("Not a screenblock position: ({}, {})", x, y),
        }
    }
}

pub struct World {
    world_data: Option<&'static WorldData>,
    frame_count: i32,
    current_room_x: usize,
    current_room_y: usize,
    skybox_anim: SkyboxBehavior,
}

impl World {
    pub fn new() -> Self {
        let bg1_settings = BackgroundControlSetting::new()
            .with_screen_base_block(WORLD_SCREENBLOCK_BG_ID)
            .with_char_base_block(WORLD_CHARBLOCK_ID)
            .with_mosaic(true)
            .with_bg_priority(2)
            .with_size(BGSize::One);

        BG1CNT.write(bg1_settings);

        let bg2_settings = BackgroundControlSetting::new()
            .with_screen_base_block(WORLD_SCREENBLOCK_SKYBOX_ID)
            .with_char_base_block(WORLD_CHARBLOCK_SPECIAL_ID)
            .with_mosaic(true)
            .with_bg_priority(3)
            .with_size(BGSize::Zero);

        BG2CNT.write(bg2_settings);

        let bg3_settings = BackgroundControlSetting::new()
            .with_screen_base_block(WORLD_SCREENBLOCK_FG_ID)
            .with_char_base_block(WORLD_CHARBLOCK_ID)
            .with_mosaic(true)
            .with_bg_priority(1)
            .with_size(BGSize::One);

        BG3CNT.write(bg3_settings);

        World {
            world_data: None,
            frame_count: 0,
            current_room_x: 0,
            current_room_y: 0,
            skybox_anim: SkyboxBehavior::None,
        }
    }

    pub fn load_world(&mut self, data: &'static WorldData) {
        self.world_data = Some(data);

        let driver = unsafe { Driver::instance_mut() };

        if let Some(song_id) = data.music.0.first() {
            driver.audio().set_bgm(&MUSIC_DATA[*song_id as usize]);
        }

        let renderer = driver.video();
        renderer.load_world_palettes(&data.pal);

        info!(
            "bg {} fg {} skybox {}",
            data.bg_layer.is_some(),
            data.fg_layer.is_some(),
            data.skybox_layer
                .as_ref()
                .map(|sb| {
                    if let RoomData::Text(_) = sb.map {
                        "text"
                    } else {
                        "affine"
                    }
                })
                .unwrap_or("none"),
        );

        let dispcnt_tmp = DisplayControlSetting::new()
            .with_bg0(true) // HUD
            .with_bg1(data.bg_layer.is_some())
            .with_bg2(data.skybox_layer.is_some())
            .with_bg3(data.fg_layer.is_some())
            .with_obj(true)
            .with_win0(false)
            .with_win1(false);

        // making up for alignment in screenblock copying.
        BG1HOFS.write(BG_HOFS_BASE);
        BG1VOFS.write(BG_VOFS_BASE);
        BG2HOFS.write(BG_HOFS_BASE);
        BG2VOFS.write(BG_VOFS_BASE);
        BG3HOFS.write(BG_HOFS_BASE);
        BG3VOFS.write(BG_VOFS_BASE);

        match data.img {
            TilePatterns::Text(imgs) => {
                renderer.load_bg_tiles(WORLD_CHARBLOCK_ID, &imgs[..512.min(imgs.len())]);
            }
            TilePatterns::TextLz77(data) => {
                renderer.load_bg_tiles_lz77(WORLD_CHARBLOCK_ID, data);
            }
            TilePatterns::AffineLz77(_) | TilePatterns::Affine(_) => {
                panic!("Tried to load world with 8bpp main images, not supported");
            }
        }

        match data.img_special {
            TilePatterns::Text(imgs) => {
                DISPCNT.write(dispcnt_tmp.with_mode(DisplayMode::Mode0));
                renderer.load_bg_tiles(WORLD_CHARBLOCK_SPECIAL_ID, &imgs[..512.min(imgs.len())]);
            }
            TilePatterns::Affine(imgs) => {
                DISPCNT.write(dispcnt_tmp.with_mode(DisplayMode::Mode1));
                renderer.load_bg_tiles(WORLD_CHARBLOCK_SPECIAL_ID, &imgs[..256.min(imgs.len())]);
            }
            TilePatterns::TextLz77(data) => {
                DISPCNT.write(dispcnt_tmp.with_mode(DisplayMode::Mode0));
                renderer.load_bg_tiles_lz77(WORLD_CHARBLOCK_SPECIAL_ID, data);
            }
            TilePatterns::AffineLz77(data) => {
                DISPCNT.write(dispcnt_tmp.with_mode(DisplayMode::Mode1));
                renderer.load_bg_tiles_lz77(WORLD_CHARBLOCK_SPECIAL_ID, data);
            }
        }

        let bg1_settings = BG1CNT.read();
        let bg2_settings = BG2CNT.read();
        if data.id == WorldId::TomsDiner {
            BG1CNT.write(bg1_settings);
            BG2CNT.write(bg2_settings.with_size(BGSize::Zero));
            BLDCNT.write(
                ColorEffectSetting::new()
                    .with_bg1_1st_target_pixel(true)
                    .with_bg2_2nd_target_pixel(true)
                    .with_color_special_effect(ColorSpecialEffect::AlphaBlending),
            );
            BLDALPHA.write(
                AlphaBlendingSetting::new()
                    .with_eva_coefficient(4)
                    .with_evb_coefficient(12),
            );


            self.skybox_anim = SkyboxBehavior::Windmill;
        }

        self.draw_skybox();
    }

    pub fn draw_skybox(&mut self) {
        let sb_addr = ScreenblockAddress::Skybox;
        sb_addr.blocks_as_mut_slice(2).fill(0u16);

        if let Some(data) = self.world_data {
            if let Some(layer) = data.skybox_layer.as_ref() {
                let rows = layer.meta.0.len();
                if rows != 0 {
                    let cols = layer.meta.0[0].len();
                    debug!("{} skybox: ({}, {})", data.name, cols, rows);
                    for row in 0..rows {
                        for col in 0..cols {
                            let sb_corner = ScreenblockCorner::from((col, row));
                            self.draw_room_layer(sb_addr, sb_corner, layer, row, col);
                        }
                    }
                }
            }
        }
    }

    fn write_row<T: Copy>(&self, sb_addr: usize, row: usize, entries: &[T]) {
        unsafe {
            CoreLib::copy_slice_to_address(entries, sb_addr + (row * 64));
            /*
            if size_of::<T>() == 2 {
                // 4bpp. u16
            } else {
                // 8bpp, u8
            }
            */
        }
    }

    fn draw_room_layer(
        &self,
        sb_addr: ScreenblockAddress,
        sb_corner: ScreenblockCorner,
        layer: &Layer,
        room_row: usize,
        room_col: usize,
    ) {
        let Layer { meta, map } = layer;

        let meta_width = meta.0.get(0).map(|x| x.len()).unwrap_or_default();
        if room_row < meta.0.len() && room_col < meta_width {
            let room_id = meta.0[room_row][room_col];
            match map {
                RoomData::Text(map) => {
                    let src = &map[room_id as usize].0;
                    let sb_addr = sb_addr as usize + sb_corner.text_offset();
                    for (row, entries) in src.iter().enumerate().take(ROOM_TILES) {
                        self.write_row(sb_addr, row, entries);
                    }
                }
                RoomData::Affine(map) => {
                    let src = &map[room_id as usize].0;
                    //let sb_addr = sb_addr as usize + sb_corner.affine_offset();
                    let sb_slice = sb_addr.blocks_as_mut_slice(1);
                    for (row, entries) in src.iter().enumerate().take(ROOM_TILES/2) {
                        let start = row * entries.len();
                        let end = start + entries.len();
                        sb_slice[start..end].copy_from_slice(entries);
                    }
                }
                RoomData::TextLz77(map) => {
                    let src = map[room_id as usize];
                    let sb_addr = sb_addr as usize + sb_corner.text_offset();
                    gba::bios::lz77_uncomp_16bit(src.as_ptr(), sb_addr as *mut u16);
                }
            }
        }
    }

    pub fn draw_room(&mut self, room_row: usize, room_col: usize) {
        debug!("drawing room: row {}, col {}", room_row, room_col);
        // TODO: refactor (draw_current_room separate from a mut set_current_room)
        self.current_room_x = room_col;
        self.current_room_y = room_row;
        if let Some(world_data) = self.world_data {
            if let Some(layer) = &world_data.bg_layer {
                self.draw_room_layer(
                    ScreenblockAddress::Background,
                    ScreenblockCorner::TopLeft,
                    layer,
                    room_row,
                    room_col,
                );
                self.draw_room_layer(
                    ScreenblockAddress::Background,
                    ScreenblockCorner::TopRight,
                    layer,
                    room_row,
                    room_col + 1,
                );
            }
            if let Some(layer) = &world_data.fg_layer {
                self.draw_room_layer(
                    ScreenblockAddress::Foreground,
                    ScreenblockCorner::TopLeft,
                    layer,
                    room_row,
                    room_col,
                );
            }
        }
    }

    pub fn dimensions(&self) -> (usize, usize) {
        let meta = &self
            .world_data
            .as_ref()
            .expect("gfx")
            .bg_layer
            .as_ref()
            .expect("bg_layer")
            .meta;
        let rows = meta.0.len();
        let cols = meta.0.get(0).map(|x| x.len()).unwrap_or_default();
        (cols, rows)
    }

    pub fn advance_frame(&mut self) {
        self.frame_count += 1;
        match self.skybox_anim {
            SkyboxBehavior::None => {
                BG2HOFS.write(BG_HOFS_BASE);
                BG2VOFS.write(BG_VOFS_BASE);
            }
            SkyboxBehavior::Windmill => {
                let angle = (self.frame_count << 7) as u16;
                let params = BgAffineSetParams {
                    data_center_x: 64 << 8,
                    data_center_y: 64 << 8,
                    display_center_x: 120,
                    display_center_y: 80,
                    scale_x: 0b10010000,
                    scale_y: 0b10010000,
                    angle,
                };
                gba::bios::bg_affine_set(&params, 0x400_0020usize, 1);
                let x = (512 - (self.frame_count & 1023)).abs() as u16;
                BG1HOFS.write(x);

                let bg1_settings = BG1CNT.read();
                let alpha = (32 - ((self.frame_count >> 3) & 63)).abs() as u16;
                if alpha < 16 {
                    BG1CNT.write(bg1_settings.with_char_base_block(WORLD_CHARBLOCK_ID))
                } else {
                    BG1CNT.write(bg1_settings.with_char_base_block(WORLD_CHARBLOCK_ID))
                }
                BLDALPHA.write(
                    AlphaBlendingSetting::new()
                        .with_eva_coefficient(16 - (alpha >> 1))
                        .with_evb_coefficient(alpha >> 1),
                );
            }
        }
    }
}
