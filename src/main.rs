#![no_std]
#![no_main]
#![feature(asm)]
#![feature(panic_info_message)]
#![feature(const_fn_transmute)]
#![feature(const_in_array_repeat_expressions)]
#![feature(const_fn)]

#![allow(clippy::new_without_default)]

#[macro_use]
extern crate flowergal_runtime;

pub mod hud;
pub mod world;

use core::fmt::Write;

use gba::io::keypad::KeyInput;
use gba::mgba::MGBADebugLevel;

use bstr::ByteSlice;
use heapless::consts::U80;

use flowergal_runtime::Driver;
use flowergal_proj_config::sound_info::{SAMPLE_RATE, CYCLES_PER_FRAME};

static mut G_HUD: Option<hud::Hud> = None;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(mut mgba) = gba::mgba::MGBADebug::new() {
        let _ = write!(mgba, "{}", info);
        mgba.send(MGBADebugLevel::Fatal);
    } else {
        unsafe {
            Driver::instance_mut().video().set_textbox_shown(true);
            if let Some(h) = &mut G_HUD {
                let mut buf: heapless::Vec<u8, U80> = heapless::Vec::new();
                let _ = write!(buf, "{}", info);
                h.clear_text_area();
                h.draw_text(buf.to_str_unchecked());
            }
        }
    }
    unsafe { Driver::instance_mut().audio().disable() };
    loop {
        gba::bios::vblank_interrupt_wait();
    }
}

#[no_mangle]
fn main() -> ! {
    debug!("Initializing");

    let driver = unsafe { Driver::instance_mut() };
    driver.initialize();

    debug!("Initialized. Drawing HUD...");

    let h = unsafe {
        G_HUD.replace(hud::Hud::new());
        G_HUD.as_mut().unwrap()
    };

    h.draw_background();

    debug!("HUD drawn. Loading World...");

    let mut w = world::World::new();
    w.load_world(&flowergal_proj_assets::TOMS_DINER_DATA);
    w.draw_room(0, 0);

    info!("World loaded.");

    let mut text_showing = true;
    driver.video().set_textbox_shown(text_showing);

    let mut prev_keys = KeyInput::new();
    loop {
        let cur_keys = gba::io::keypad::read_key_input();
        let new_keys = cur_keys.pressed_since(prev_keys);

        if new_keys.a() {
            text_showing = !text_showing;
            driver.video().set_textbox_shown(text_showing);
            h.clear_text_area();
        }

        if text_showing {
            let mut buf: heapless::Vec<u8, U80> = heapless::Vec::new();
            let dec = driver.audio().ticks_decode * 100 / (CYCLES_PER_FRAME / 64);
            let mix = driver.audio().ticks_unmix * 100 / (CYCLES_PER_FRAME / 64);
            let _ = write!(
                buf,
                "9-bit FLAC @ {}Hz\n\
                CPU: {:2}% dec,{:2}% mix\n\
                Rust+ASM by lifning",
                SAMPLE_RATE,
                dec.min(99), // formatting gets screwed on GBARunner2
                mix.min(99));
            h.clear_text_area();
            h.draw_text(unsafe { buf.to_str_unchecked() });
        }

        if new_keys.select() {
            if let Some(mut mgba) = gba::mgba::MGBADebug::new() {
                for s in flowergal_proj_assets::LICENSE_TEXT.lines() {
                    let _ = mgba.write_str(s);
                    mgba.send(MGBADebugLevel::Info);
                }
            }
        }

        prev_keys = cur_keys;
        w.advance_frame();
        h.draw_borders(text_showing);

        gba::bios::vblank_interrupt_wait();

        driver.audio().mixer();
    }
}
