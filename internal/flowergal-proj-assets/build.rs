use flowergal_buildtools::music::pcm_conv;
use flowergal_buildtools::{user_interface, world_data_gen};
use flowergal_buildtools::license_text;

fn main() {
    license_text::generate_text().expect("Couldn't collect dependency license files");
    user_interface::convert_assets().expect("Couldn't convert assets for UI");
    world_data_gen::convert_world_maps().expect("Couldn't convert level maps from original assets");
    pcm_conv::convert_songs_and_sfx().expect("Couldn't convert audio files");
}
