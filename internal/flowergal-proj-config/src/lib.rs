#![cfg_attr(target_arch = "arm", no_std)]
#![feature(const_fn)]
#![feature(const_fn_transmute)]
#![feature(const_raw_ptr_deref)]
#![feature(const_slice_from_raw_parts)]

#[macro_use]
extern crate static_assertions;

pub mod resources;

pub mod world_info;
pub use world_info::*;

pub mod sound_info;

#[macro_use]
pub mod macros;
