use flowergal_proj_config::resources::{RoomData, TextScreenblockEntry, TilePatterns, ROOM_SIZE};
use std::error::Error;
use std::mem::size_of;
use std::slice::from_raw_parts;

pub trait CompressibleAsset: Sized {
    fn compress(self) -> Result<Self, Box<dyn Error>>;
}

#[allow(clippy::size_of_in_element_count)]
pub fn do_lz77_compression<T>(input: &[T], vram_safe: bool) -> Result<&'static [u32], Box<dyn Error>> {
    let input =
        unsafe { from_raw_parts(input.as_ptr() as *const u8, input.len() * size_of::<T>()) };
    let mut data = gba_compression::bios::compress_lz77(input, vram_safe)?;
    while data.len() & 3 != 0 {
        data.push(0);
    }
    let leaked_data = Box::leak(data.into_boxed_slice());
    let aligned_leaked_data = unsafe {
        from_raw_parts(
            leaked_data.as_ptr() as *const u32,
            leaked_data.len() / size_of::<u32>(),
        )
    };
    Ok(aligned_leaked_data)
}

impl CompressibleAsset for TilePatterns {
    fn compress(self) -> Result<Self, Box<dyn Error>> {
        match self {
            TilePatterns::Text(tiles) => {
                Ok(TilePatterns::TextLz77(do_lz77_compression(tiles, true)?))
            }
            TilePatterns::Affine(tiles) => {
                Ok(TilePatterns::AffineLz77(do_lz77_compression(tiles, true)?))
            }
            x => Ok(x),
        }
    }
}

impl CompressibleAsset for RoomData {
    fn compress(self) -> Result<Self, Box<dyn Error>> {
        match self {
            RoomData::Text(rooms) => {
                let mut compressed_rooms = Vec::<&'static [u32]>::new();
                for room in rooms {
                    let mut padded = Vec::new();
                    for (i, row) in room.0.iter().enumerate() {
                        for entry in row.iter() {
                            padded.push(*entry);
                        }
                        if i < ROOM_SIZE.1 - 1 {
                            for _ in (ROOM_SIZE.0)..32 {
                                padded.push(TextScreenblockEntry::new());
                            }
                        }
                    }
                    compressed_rooms.push(do_lz77_compression(&padded, true)?);
                }
                Ok(RoomData::TextLz77(Box::leak(
                    compressed_rooms.into_boxed_slice(),
                )))
            }
            // TODO: support.  for now, passthrough is OK
            //RoomData::Affine(_) => Err("compressing affine RoomData not yet supported".into()),
            x => Ok(x),
        }
    }
}
