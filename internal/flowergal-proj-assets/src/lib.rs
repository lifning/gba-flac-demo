#![no_std]
#![feature(const_fn_transmute)]

#[macro_use]
extern crate build_const;

#[macro_use]
extern crate flowergal_proj_config;

use flowergal_proj_config::resources::*;
use flowergal_proj_config::sound_info::{MusicId, TrackList};
use flowergal_proj_config::world_info::WorldId::*;

build_const!("world_gfx_bc");
build_const!("ui_gfx_bc");
build_const!("sound_data_bc");
build_const!("license_text_bc");
