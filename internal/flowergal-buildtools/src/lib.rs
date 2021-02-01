// Copyright (C) 2021 lifning, licensed under the GNU Affero General Public License version 3.

#![feature(seek_convenience)]
#![feature(generators)]
#![feature(drain_filter)]
#![feature(command_access)]

#![allow(clippy::comparison_chain)]
#![allow(clippy::new_without_default)]
#![allow(clippy::ptr_arg)]

pub mod compression;
pub mod music;
pub mod tile_generation;
pub mod user_interface;
pub mod world_data_gen;
pub mod license_text;
