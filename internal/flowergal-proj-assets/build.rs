use flowergal_buildtools::music::pcm_conv;
use flowergal_buildtools::{user_interface, world_data_gen};
use flowergal_buildtools::license_text;

trait NiceExpect {
    fn nice_expect(self, msg: &str);
}

impl<T, E: std::fmt::Display> NiceExpect for Result<T, E> {
    fn nice_expect(self, msg: &str) {
        if let Err(e) = self {
            eprintln!("{}", e);
            panic!("{}", msg);
        }
    }
}

fn main() {
    pcm_conv::convert_songs_and_sfx().nice_expect("Couldn't convert audio files");
    license_text::generate_text().nice_expect("Couldn't collect dependency license files");
    user_interface::convert_assets().nice_expect("Couldn't convert assets for UI");
    world_data_gen::convert_world_maps().nice_expect("Couldn't convert level maps from original assets");
}
