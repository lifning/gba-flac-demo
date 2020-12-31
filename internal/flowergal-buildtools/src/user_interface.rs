use std::error::Error;
use std::path::Path;

use sdl2::image::LoadSurface;
use sdl2::surface::Surface;

use build_const::ConstWriter;

use crate::tile_generation;
use flowergal_proj_config::resources::TilePatterns;

const ASSET_DIR: &str = "../../assets/gfx";
const ASSETS: &[&str] = &["hud.png", "font.png", "textbox.png"];

pub fn convert_assets() -> Result<(), Box<dyn Error>> {
    let _sdl_context = sdl2::init()?;
    let _image_context = sdl2::image::init(sdl2::image::InitFlag::PNG)?;

    let mut bc_out = ConstWriter::for_build("ui_gfx_bc")?.finish_dependencies();

    let mut surfs = Vec::new();
    for asset_name in ASSETS {
        let surf_path = Path::new(ASSET_DIR).join(asset_name);
        println!("cargo:rerun-if-changed={}", surf_path.to_string_lossy());
        surfs.push(Surface::from_file(&surf_path)?);
    }

    // TODO: eventually keep track of multi-palette / flip reduction etc.? not necessary yet
    let (_grids, bank) = tile_generation::process_basic_tilesets(surfs, 16)?;

    if let TilePatterns::Text(img) = bank.gba_patterns(false) {
        let pal = bank.gba_palette_full();
        bc_out.add_array("UI_PAL", "Color", &pal.data());
        bc_out.add_array("UI_IMG", "Tile4bpp", img);
    } else {
        return Err("Found UI assets in 8bpp format, unsupported".into());
    }

    bc_out.finish();

    Ok(())
}
