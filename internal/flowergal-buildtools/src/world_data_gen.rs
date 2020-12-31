// Copyright (C) 2021 lifning, licensed under the GNU Affero General Public License version 3.

use std::error::Error;
use std::path::Path;

use build_const::ConstWriter;

use rayon::prelude::*;

use sdl2::image::LoadSurface;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;

use crate::compression::CompressibleAsset;
use crate::tile_generation::{sdl_support, ImageBank, Grid};
use flowergal_proj_config::resources::blend::float;
use flowergal_proj_config::resources::{ColorEffectType, Layer, PaletteData, WorldData, WorldPalettes, BLEND_ENTRIES, TEXTBOX_A, TEXTBOX_B, TEXTBOX_G, TEXTBOX_R, TEXTBOX_Y_MID_EFFECT_INDEX, ROOM_SIZE};
use flowergal_proj_config::{WorldId, WorldResourceInfo, WORLD_RESOURCE_INFO};

const ASSET_DIR: &str = "../../assets/gfx";
const TILEMAPS_DIR: &str = "../../assets/tilemaps";
const ANIMTILES_DIR: &str = "../../assets/animtiles";
const SKYBOX_DIR: &str = "../../assets/skybox";
const EFFECT_DIR: &str = "../../assets/effect";

fn convert_single_world_map(world: &WorldResourceInfo) -> String {
    let mod_name = format!("{}_gfx_bc", world.name);
    let mut out = ConstWriter::for_build(&mod_name)
        .unwrap()
        .finish_dependencies();

    let data = generate_world_data(world)
        .map_err(|e| format!("{}: {}", world.name, e))
        .unwrap();

    out.add_value(&format!("{}_DATA", world.name), "WorldData", data);
    out.finish();
    mod_name
}

pub fn convert_world_maps() -> Result<(), Box<dyn Error>> {
    let _sdl_context = sdl2::init().expect("Couldn't init SDL2");
    let _image_context =
        sdl2::image::init(sdl2::image::InitFlag::PNG).expect("Couldn't init SDL2_image");

    let mut bc_out = ConstWriter::for_build("world_gfx_bc")?.finish_dependencies();
    let mod_name_results: Vec<String> = WORLD_RESOURCE_INFO
        .par_iter()
        .map(convert_single_world_map)
        .collect();
    for mod_name_res in mod_name_results {
        bc_out.add_raw(&format!(r#"build_const!("{}");"#, mod_name_res));
    }

    bc_out.finish();

    Ok(())
}

pub fn render_world_to_16bit_surfaces(
    world: &WorldResourceInfo,
) -> Result<Vec<Surface>, Box<dyn Error>> {
    let orig = load_surface_resource(TILEMAPS_DIR, world.tilemap_path)?;
    let format = PixelFormatEnum::ARGB1555;
    let mut dst = Surface::new(orig.width(), orig.height(), format)?;
    let dst_rect = dst.rect();
    orig.blit(orig.rect(), &mut dst, dst_rect)?;
    Ok(vec![dst])
}

fn load_surface_resource<'a>(
    base_dir: &str,
    filename: &str,
) -> Result<Surface<'a>, Box<dyn Error>> {
    let surf_path = [ASSET_DIR, base_dir]
        .iter()
        .map(|s| Path::new(s).join(filename))
        .find(|p| p.is_file())
        .ok_or(format!(
            "{} not found in {} or override dir",
            filename, base_dir
        ))?;
    println!("cargo:rerun-if-changed={}", surf_path.to_string_lossy());
    Ok(Surface::from_file(&surf_path)?)
}

fn generate_blend_palettes(
    gba_pal_normal: &[gba::Color],
    blend_colors: &Vec<sdl2::pixels::Color>,
    blend_function: fn(gba::Color, (u8, u8, u8)) -> gba::Color,
) -> Vec<&'static [gba::Color]> {
    let num_palettes = BLEND_ENTRIES;
    let mut palettes: Vec<&'static [gba::Color]> = Vec::with_capacity(num_palettes);
    for i in 0..num_palettes {
        let sdl_c = blend_colors
            [(blend_colors.len() - 1).min(i * blend_colors.len() / (num_palettes - 1))]
        .rgb();
        let p: Vec<gba::Color> = gba_pal_normal
            .iter()
            .map(|gba_c| blend_function(*gba_c, sdl_c))
            .collect();
        palettes.push(Box::leak(p.into_boxed_slice()));
    }
    palettes
}

fn generate_animtiles(
    world: &WorldResourceInfo,
    bank: &mut ImageBank,
) -> Result<Grid, Box<dyn Error>> {
    if let Some(anim_path) = world.anim_path {
        let surf = load_surface_resource(ANIMTILES_DIR, anim_path)?;
        let blend = match world.id {
            _ => true,
        };
        let grid = bank.process_image_region(&surf, None, true, false, blend)?;
        Ok(grid)
    } else {
        Ok(Grid::new((0, 0), true))
    }
}

fn generate_skybox(
    world: &WorldResourceInfo,
    bank: &mut ImageBank,
) -> Result<Option<Layer>, Box<dyn Error>> {
    if let Some(skybox_path) = world.skybox_path {
        let surf = load_surface_resource(SKYBOX_DIR, skybox_path)?;
        let tile8_is_blended = match world.id {
            _ => |_, _| true,
        };
        let rb = bank.process_world_map(&surf, true, &tile8_is_blended, (16, 16))?;

        // FIXME/HACK: at this point we happen to have all the colors
        let pal_bank_ofs = bank.palette_bank.palettes_blend.len();
        let (meta, map) = rb.gba_metamap_and_roomdata(pal_bank_ofs);
        Ok(Some(Layer {
            map: map.compress()?,
            meta,
        }))
    } else {
        Ok(None)
    }
}

fn compute_blend_effects(
    world: &WorldResourceInfo,
    bank: &mut ImageBank,
) -> Result<WorldPalettes, Box<dyn Error>> {
    let normal_palette = bank.gba_palette_full();
    let blend_len = bank.blend_palette_size();

    let blended_palettes =
        Box::leak(compute_overlay_palettes(world, &normal_palette.data()[..blend_len])?
            .into_boxed_slice());

    let mut base_pal = Vec::from(normal_palette.data());
    for (base_col, blend_col) in base_pal.iter_mut()
        .zip(blended_palettes.get(TEXTBOX_Y_MID_EFFECT_INDEX).map(|x| x.data().iter()).unwrap_or([].iter()))
    {
        *base_col = *blend_col;
    }
    let base_pal = PaletteData::new(base_pal.leak());
    let textbox_blend_palette = compute_textbox_palette(&base_pal);

    Ok(WorldPalettes {
        normal_palette,
        blended_palettes,
        textbox_blend_palette,
        // gradient_colors: PaletteData(Box::leak(sdl_support::sdl_to_gba_colors(blend_colors).into_boxed_slice())),
    })
}

fn compute_textbox_palette(base_pal: &PaletteData) -> PaletteData {
    let textbox_rgb = (TEXTBOX_R as u8, TEXTBOX_G as u8, TEXTBOX_B as u8);
    let alpha = TEXTBOX_A as f64 / 255.0;
    let text_pal: Vec<gba::Color> = base_pal
        .data()
        .iter()
        .map(|a| float::blend_alpha(*a, textbox_rgb, alpha))
        .collect();
    PaletteData::new(Box::leak(text_pal.into_boxed_slice()))
}

fn compute_overlay_palettes(
    world: &WorldResourceInfo,
    gba_pal_normal: &[gba::Color],
) -> Result<Vec<PaletteData>, Box<dyn Error>> {
    if let Some(effect_path) = world.effect_path {
        let surf = load_surface_resource(EFFECT_DIR, effect_path)?;
        let mut rect = surf.rect();
        rect.set_x(rect.width() as i32 / 3);
        rect.set_width(1);
        // note: backdrop of gray seems to be a no-op color for overlay blend
        //  (instead of implementing alpha compositing again)
        let blend_colors =
            sdl_support::pixels_of_rect_32bit(&surf, rect, Some(sdl2::pixels::Color::GRAY))?;

        let blend_function = match world.effect_type {
            ColorEffectType::Overlay => float::blend_overlay,
            ColorEffectType::HardLight => float::blend_hardlight,
            ColorEffectType::Multiply => float::blend_multiply,
            ColorEffectType::None => {
                return Err("Specified an effect_path, but no effect_type".into());
            }
        };
        let palettes = generate_blend_palettes(&gba_pal_normal, &blend_colors, blend_function)
            .into_iter()
            .map(|x| PaletteData::new(x))
            .collect();

        Ok(palettes)
    } else if let ColorEffectType::None = world.effect_type {
        Ok(Vec::new())
    } else {
        Err("Specified an effect_type, but no effect_path".into())
    }
}

fn generate_world_data(world: &WorldResourceInfo) -> Result<WorldData, Box<dyn Error>> {
    let special_is_4bpp = world.id != WorldId::TomsDiner;
    let max_colors = 240;

    let mut bank = ImageBank::new(max_colors, special_is_4bpp);
    let mut rooms = Vec::new();

    // render actual level maps
    let map_surfs = render_world_to_16bit_surfaces(world)?;
    for surf in map_surfs.into_iter() {
        let tile8_is_blended = |_, _| true;
        rooms.push(bank.process_world_map(&surf, false, tile8_is_blended, ROOM_SIZE)?);
    }
    // animated tiles... might get crowded
    // TODO: save grid's.. grid
    let _anim = generate_animtiles(world, &mut bank)?;

    // FIXME: subtle: starts converting SbEntries on its own, needs refactor
    let skybox_layer = generate_skybox(world, &mut bank)?;

    let pal_bank_ofs = bank.palette_bank.palettes_blend.len();

    let mut bg_fg_layers: Vec<Layer> = rooms
        .iter()
        .map(|b| b.gba_metamap_and_roomdata(pal_bank_ofs))
        .map(|(meta, map)| Layer {
            map: map.compress().unwrap(),
            meta,
        })
        .collect();
    let bg_layer = bg_fg_layers.remove(0).into();
    let fg_layer = bg_fg_layers.pop();

    let pal = compute_blend_effects(world, &mut bank)?;

    let img = bank.gba_patterns(false).compress()?;
    let img_special = bank.gba_patterns(true).compress()?;

    let world_data = WorldData {
        id: world.id,
        name: world.name,
        pal,
        img,
        img_special,
        bg_layer,
        fg_layer,
        skybox_layer,
        // TODO anim: Box::leak(anim.into_boxed_slice()), anim_grid
        music: world.songs.clone(),
    };

    Ok(world_data)
}
