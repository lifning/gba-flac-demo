/*
 * Drivers, codecs, platform abstractions.
 * Eventual goal is to have game-specific stuff only where pragmatic or necessary.
 */

#![no_std]

#![feature(asm)]
#![feature(unchecked_math)]
#![feature(fixed_size_array)]
#![feature(array_windows)]
#![feature(slice_fill)]
#![feature(slice_fill_with)]
#![feature(const_fn_transmute)]
#![feature(const_mut_refs)]
#![feature(const_in_array_repeat_expressions)]
#![feature(const_fn)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(const_slice_from_raw_parts)]
#![feature(const_raw_ptr_deref)]
#![feature(iterator_fold_self)]
#![feature(associated_type_defaults)]
#![feature(isa_attribute)]
#![feature(fmt_as_str)]

#![allow(clippy::missing_safety_doc)]

#[macro_use]
pub mod logging;
pub mod audio;
pub mod interrupt_service;
pub mod memory;
pub mod render;
pub mod timers;

use crate::audio::AudioDriver;
use crate::render::GbaRenderer;
use crate::timers::GbaTimer;
pub use memory::{BiosCalls, MemoryOps, CoreLib};

pub struct Driver {
    video: render::GbaRenderer,
    audio: audio::AudioDriver,
    timer: timers::GbaTimer,
    _input: (),
    _interrupt: (),
}

static mut DRIVER_SINGLETON: Driver = Driver::new();

impl Driver {
    pub const fn new() -> Self {
        Driver {
            video: render::GbaRenderer::new(),
            audio: audio::AudioDriver::new(),
            timer: timers::GbaTimer::new(),
            _input: (),
            _interrupt: (),
        }
    }

    // TODO: make safe w/ locking mechanism (per subsystem?) so ISR's don't screw things up
    #[inline(always)]
    pub unsafe fn instance_mut() -> &'static mut Self {
        &mut DRIVER_SINGLETON
    }

    #[inline(always)]
    pub fn video(&mut self) -> &mut GbaRenderer {
        &mut self.video
    }

    #[inline(always)]
    pub fn audio(&mut self) -> &mut AudioDriver {
        &mut self.audio
    }

    #[inline(always)]
    pub fn timer(&mut self) -> &mut GbaTimer {
        &mut self.timer
    }

    pub fn initialize(&mut self) {
        self.video.initialize();
        self.audio.initialize();
        self.timer.initialize();
        interrupt_service::irq_setup();
    }
}
