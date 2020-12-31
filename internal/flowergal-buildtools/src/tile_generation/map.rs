use gba::vram::affine::AffineScreenblockEntry;
use gba::vram::text::TextScreenblockEntry;

use flowergal_proj_config::resources::{
    Metamap, RoomData, RoomEntries4bpp, RoomEntries8bpp, ROOM_SIZE,
};

use super::{TILE_H, TILE_W};
use crate::tile_generation::PalBankId;

#[derive(Eq, PartialEq, Clone)]
pub struct SbEntry {
    pub tile_num: usize,
    pub hflip: bool,
    pub vflip: bool,
    pub palbank: PalBankId,
}

impl Default for SbEntry {
    fn default() -> Self {
        SbEntry {
            tile_num: 42,
            hflip: true,
            vflip: true,
            palbank: PalBankId::Blended(42),
        }
    }
}

impl SbEntry {
    pub fn to_text_sbe(&self, pal_bank_ofs: usize) -> TextScreenblockEntry {
        let palbank = self.palbank.bake(pal_bank_ofs);
        assert!(self.tile_num < 512);
        assert!(palbank < 16);
        TextScreenblockEntry::from_tile_id(self.tile_num as u16)
            .with_hflip(self.hflip)
            .with_vflip(self.vflip)
            .with_palbank(palbank)
    }
    pub fn to_affine_sbe(&self) -> AffineScreenblockEntry {
        assert!(self.tile_num < 256);
        assert!(!self.hflip);
        assert!(!self.vflip);
        assert_eq!(self.palbank, PalBankId::Blended(0));
        AffineScreenblockEntry(self.tile_num as u8)
    }
}

#[derive(Clone)]
pub struct Grid {
    pub sb_entries: Vec<Vec<SbEntry>>,
    pub grid_width: usize,
    pub grid_height: usize,
    pub is_4bpp: bool,
}

impl Grid {
    pub fn new(grid_size: (usize, usize), is_4bpp: bool) -> Self {
        let (grid_width, grid_height) = grid_size;
        Grid {
            sb_entries: vec![vec![SbEntry::default(); grid_width]; grid_height],
            grid_width,
            grid_height,
            is_4bpp,
        }
    }

    pub fn set_sbe(&mut self, tx: usize, ty: usize, sbe: SbEntry) {
        self.sb_entries[ty][tx] = sbe;
    }

    pub fn gba_raw_sbe_4bpp(&self, pal_bank_ofs: usize) -> Vec<TextScreenblockEntry> {
        assert!(self.is_4bpp);
        let mut entries = Vec::new();
        for data in self.sb_entries.iter() {
            for ent in data.iter() {
                entries.push(ent.to_text_sbe(pal_bank_ofs));
            }
        }
        entries
    }

    pub fn gba_raw_sbe_8bpp(&self) -> Vec<AffineScreenblockEntry> {
        assert!(!self.is_4bpp);
        let mut entries = Vec::new();
        for data in self.sb_entries.iter() {
            for ent in data.iter() {
                entries.push(ent.to_affine_sbe());
            }
        }
        entries
    }

    pub fn gba_room_entries_4bpp(&self, pal_bank_ofs: usize) -> RoomEntries4bpp {
        assert!(self.is_4bpp);
        assert_eq!(self.grid_width, ROOM_SIZE.0);
        assert_eq!(self.grid_height, ROOM_SIZE.1);
        let mut array = [[TextScreenblockEntry::default(); ROOM_SIZE.0]; ROOM_SIZE.1];
        for (row, data) in self.sb_entries.iter().enumerate() {
            for (col, ent) in data.iter().enumerate() {
                array[row][col] = ent.to_text_sbe(pal_bank_ofs);
            }
        }
        RoomEntries4bpp(array)
    }

    pub fn gba_room_entries_8bpp(&self) -> RoomEntries8bpp {
        assert!(!self.is_4bpp);
        assert_eq!(self.grid_width, ROOM_SIZE.0/2);
        assert_eq!(self.grid_height, ROOM_SIZE.1/2);
        let mut array = [[AffineScreenblockEntry::default(); ROOM_SIZE.0/2]; ROOM_SIZE.1/2];
        for (row, data) in self.sb_entries.iter().enumerate() {
            for (col, ent) in data.iter().enumerate() {
                array[row][col] = ent.to_affine_sbe();
            }
        }
        RoomEntries8bpp(array)
    }
}

pub struct RoomBank {
    pub rooms: Vec<Vec<Grid>>,
    pub room_width: usize,
    pub room_height: usize,
    pub map_width: usize,
    pub map_height: usize,
    pub is_4bpp: bool,
}

impl RoomBank {
    pub fn new(map_size_pixels: (u32, u32), room_size: (usize, usize), is_4bpp: bool) -> Self {
        let (room_width, room_height) = room_size;
        let map_width = map_size_pixels.0 as usize / TILE_W;
        let map_height = map_size_pixels.1 as usize / TILE_H;
        let rooms = vec![
            vec![Grid::new(room_size, is_4bpp); map_width / room_width];
            map_height / room_height
        ];
        RoomBank {
            rooms,
            room_width,
            room_height,
            map_width,
            map_height,
            is_4bpp,
        }
    }

    pub fn set_sbe(&mut self, tx: usize, ty: usize, sbe: SbEntry) {
        self.rooms[ty / self.room_height][tx / self.room_width].set_sbe(
            tx % self.room_width,
            ty % self.room_height,
            sbe,
        )
    }

    pub fn gba_metamap_and_roomdata(&self, pal_bank_ofs: usize) -> (Metamap, RoomData) {
        // could do with a refactor...
        let mut room_data_4bpp = Vec::with_capacity(self.map_width * self.map_height);
        let mut room_data_8bpp = Vec::with_capacity(self.map_width * self.map_height);
        let mut meta_outer: Vec<&[u8]> = Vec::with_capacity(self.rooms.len());
        for row in &self.rooms {
            let mut meta_inner = Vec::with_capacity(row.len());
            for room in row {
                if self.is_4bpp {
                    meta_inner.push(room_data_4bpp.len() as u8);
                    room_data_4bpp.push(room.gba_room_entries_4bpp(pal_bank_ofs));
                } else {
                    meta_inner.push(room_data_8bpp.len() as u8);
                    room_data_8bpp.push(room.gba_room_entries_8bpp());
                }
            }
            meta_outer.push(Box::leak(meta_inner.into_boxed_slice()));
        }
        let mm = Metamap(Box::leak(meta_outer.into_boxed_slice()));
        let rd = if self.is_4bpp {
            RoomData::Text(Box::leak(room_data_4bpp.into_boxed_slice()))
        } else {
            RoomData::Affine(Box::leak(room_data_8bpp.into_boxed_slice()))
        };
        (mm, rd)
    }
}
