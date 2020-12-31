#![allow(clippy::size_of_in_element_count)]

/// # GBA memory map & timing (courtesy GBATEK)
///
///  |Region      |Bus  |Read len |Write len|Cycles   |Offset range         |
///  |------------|-----|---------|---------|---------|---------------------|
///  | BIOS ROM   | 32  | 8/16/32 | -       | 1/1/1   | `000_0000-000_3fff` |
///  | IWRAM 32K  | 32  | 8/16/32 | 8/16/32 | 1/1/1   | `300_0000-300_7fff` |
///  | I/O Regs   | 32  | 8/16/32 | 8/16/32 | 1/1/1   | `400_0000-400_03fe` |
///  | OAM        | 32  | 8/16/32 |- /16/32 | 1/1/1`*`| `700_0000-700_03ff` |
///  | EWRAM 256K | 16  | 8/16/32 | 8/16/32 | 3/3/6   | `200_0000-203_ffff` |
///  | Pal. RAM   | 16  | 8/16/32 |- /16/32 | 1/1/2`*`| `500_0000-500_03ff` |
///  | VRAM       | 16  | 8/16/32 |- /16/32 | 1/1/2`*`| `600_0000-601_7fff` |
///  | Cart (WS0) | 16  | 8/16/32 |- /16/32 | 5/5/8`~`| `800_0000-9ff_ffff` |
///  | Cart (WS1) |  "  |   "     |   "     |  "      | `a00_0000-bff_ffff` |
///  | Cart (WS2) |  "  |   "     |   "     |  "      | `c00_0000-dff_ffff` |
///  | Cart SRAM  |  8  | 8/ - / -| 8/ - / -| 5/ -/ - | `e00_0000-e00_ffff` |
///
/// `*`: add one cycle if the CPU tries to access VRAM while the GBA is drawing
/// `~`: sequential accesses use a different waitstate, depending on value of WAITCNT.
///      for our value, i believe ROM is more like 4/4/6, where 6 = 1(overhead) + 4(ws0a) + 1(ws0b).
///      generally, reading n halfwords would cost 4+n cycles (plus write depending on destination).

use core::mem::size_of;
use gba::io::dma::{DMAControlSetting, DMASrcAddressControl, DMAStartTiming, DMA3};

// TODO: detect addresses in SRAM range and force those to byte-at-a-time copy

#[no_mangle]
#[no_builtins]
#[link_section = ".iwram"]
pub unsafe extern "aapcs" fn __aeabi_memcpy(mut dest: *mut u8, mut src: *const u8, mut n: usize) {
    // No matter what, we at least want half-word-alignment for our optimization paths.
    if dest as usize & 0b1 != 0 && n != 0 {
        *dest = *src;
        dest = dest.add(1);
        src = src.add(1);
        n -= 1;
    }
    match (dest as usize ^ src as usize) & 0b11 {
        // Same alignment, might just need the first half-word and then hand off to _memcpy4
        0b00 => {
            if dest as usize & 0b10 != 0 {
                asm!(
                    "subs {tmp}, {n}, #2",
                    "movge {n}, {tmp}",
                    "ldrhge {tmp}, [{src}], #2",
                    "strhge {tmp}, [{dest}], #2",
                    dest = inout(reg) dest,
                    src = inout(reg) src,
                    n = inout(reg) n,
                    tmp = out(reg) _,
                );
            }
            __aeabi_memcpy4(dest, src, n)
        },
        // Both have 2-alignment, so 16-bit register transfer is okay.
        0b10 => asm!(
            "1:",
            "subs {tmp}, {n}, #2",
            "movge {n}, {tmp}",
            "ldrhge {tmp}, [{src}], #2",
            "strhge {tmp}, [{dest}], #2",
            "bge 1b",
            "cmp {n}, #1",
            "ldrbeq {tmp}, [{src}], #1",
            "strbeq {tmp}, [{dest}], #1",
            dest = inout(reg) dest => _,
            src = inout(reg) src => _,
            n = inout(reg) n => _,
            tmp = out(reg) _,
            options(nostack)
        ),
        // Misaligned at the byte level, one byte at a time copy
        0b01 | 0b11 => for i in 0..n {
            *dest.add(i) = *src.add(i);
        }
        _ => unreachable!(),
    }
}

#[no_mangle]
#[no_builtins]
#[link_section = ".iwram"]
pub unsafe extern "aapcs" fn __aeabi_memcpy4(dest: *mut u8, src: *const u8, n: usize) {
    // We are guaranteed 4-alignment, so 32-bit register transfer is okay.
    asm!(
        "1:",
        "subs r8, {n}, #32",
        "movge {n}, r8",
        "ldmiage {src}!, {{r0-r5, r7-r8}}",
        "stmiage {dest}!, {{r0-r5, r7-r8}}",
        "bge 1b",
        "subs r8, {n}, #16",
        "movge {n}, r8",
        "ldmiage {src}!, {{r0-r3}}",
        "stmiage {dest}!, {{r0-r3}}",
        "subs r8, {n}, #8",
        "movge {n}, r8",
        "ldmiage {src}!, {{r0-r1}}",
        "stmiage {dest}!, {{r0-r1}}",
        "subs r8, {n}, #4",
        "movge {n}, r8",
        "ldrge r0, [{src}], #4",
        "strge r0, [{dest}], #4",
        "subs r8, {n}, #2",
        "movge {n}, r8",
        "ldrhge r0, [{src}], #2",
        "strhge r0, [{dest}], #2",
        "cmp {n}, #1",
        "ldrbeq r0, [{src}], #1",
        "strbeq r0, [{dest}], #1",
        dest = inout(reg) dest => _,
        src = inout(reg) src => _,
        n = inout(reg) n => _,
        out("r0") _,
        out("r1") _,
        out("r2") _,
        out("r3") _,
        out("r4") _,
        out("r5") _,
        out("r7") _,
        out("r8") _,
        options(nostack)
    );
}

#[no_mangle]
#[no_builtins]
#[link_section = ".iwram"]
pub unsafe extern "aapcs" fn __aeabi_memset(mut dest: *mut u8, mut n: usize, c: i32) {
    let byte = (c as u32) & 0xff;
    // No matter what, we at least want half-word-alignment for our optimization paths.
    if dest as usize & 0b1 != 0 && n != 0 {
        *dest = byte as u8;
        dest = dest.add(1);
        n -= 1;
    }
    let c = (byte << 24) | (byte << 16) | (byte << 8) | byte;
    if dest as usize & 0b10 != 0 {
        asm!(
            "subs {tmp}, {n}, #2",
            "movge {n}, {tmp}",
            "strhge {c}, [{dest}], #2",
            dest = inout(reg) dest,
            c = in(reg) c,
            n = inout(reg) n,
            tmp = out(reg) _,
        );
    }
    memset_word(dest as *mut u32, n, c)
}

#[no_mangle]
#[no_builtins]
#[link_section = ".iwram"]
pub unsafe extern "aapcs" fn __aeabi_memset4(dest: *mut u8, n: usize, c: i32) {
    let byte = (c as u32) & 0xff;
    let c = (byte << 24) | (byte << 16) | (byte << 8) | byte;
    memset_word(dest as *mut u32, n, c)
}

#[link_section = ".iwram"]
pub unsafe fn memset_word(dest: *mut u32, n_bytes: usize, c: u32) {
    asm!(
        "mov r1, r0",
        "mov r2, r0",
        "mov r3, r0",
        "mov r4, r0",
        "mov r5, r0",
        "mov r7, r0",
        "mov r8, r0",
        "1:",
        "subs {tmp}, {n}, #32",
        "movge {n}, {tmp}",
        "stmiage {dest}!, {{r0-r5, r7, r8}}",
        "bge 1b",
        "subs {tmp}, {n}, #16",
        "movge {n}, {tmp}",
        "stmiage {dest}!, {{r0-r3}}",
        "subs {tmp}, {n}, #8",
        "movge {n}, {tmp}",
        "stmiage {dest}!, {{r0-r1}}",
        "subs {tmp}, {n}, #4",
        "movge {n}, {tmp}",
        "strge r0, [{dest}], #4",
        "subs {tmp}, {n}, #2",
        "movge {n}, {tmp}",
        "strhge r0, [{dest}], #2",
        "subs {tmp}, {n}, #1",
        "moveq {n}, {tmp}",
        "strbeq r0, [{dest}], #1",
        dest = inout(reg) dest => _,
        n = inout(reg) n_bytes => _,
        tmp = out(reg) _,
        in("r0") c,
        out("r1") _,
        out("r2") _,
        out("r3") _,
        out("r4") _,
        out("r5") _,
        out("r7") _,
        out("r8") _,
        options(nostack)
    );
}

pub trait MemoryOps {
    unsafe fn copy_slice_to_address<T: Copy>(src: &[T], dest: usize);
    unsafe fn fill_slice_32(dest: &mut [u32], value: u32);
    unsafe fn fill_slice_16(_dest: &mut [u16], _value: u16) { unimplemented!(); }
    unsafe fn zero_block(dest: usize, count: usize) {
        let dest = core::slice::from_raw_parts_mut(dest as *mut u32, count / size_of::<u32>());
        Self::fill_slice_32(dest, 0);
    }
}

pub struct CoreLib;

impl MemoryOps for CoreLib {
    unsafe fn copy_slice_to_address<T: Copy>(src: &[T], dest: usize) {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dest as *mut T, src.len())
        // core::slice::from_raw_parts_mut(dest as *mut T, src.len()).copy_from_slice(src);
    }

    unsafe fn fill_slice_32(dest: &mut [u32], value: u32) {
        dest.fill(value);
        // core::ptr::write_bytes(dest.as_mut_ptr(), value, dest.len());
    }
}

pub struct BiosCalls;

impl MemoryOps for BiosCalls {
    unsafe fn copy_slice_to_address<T: Copy + Sized>(src: &[T], dest: usize) {
        let src_ptr = src.as_ptr() as *const u32;
        let dest_ptr = dest as *mut u32;
        let byte_count = src.len() * size_of::<T>();

        // enforce alignment
        if !cfg!(release) {
            assert_eq!(src_ptr as usize & 3, 0);
            assert_eq!(dest_ptr as usize & 3, 0);
            assert_eq!(byte_count & 3, 0);
        }

        let block_byte_count = byte_count & !31;
        let block_word_count = block_byte_count / size_of::<u32>();

        if block_word_count != 0 {
            gba::bios::cpu_fast_set(src_ptr, dest_ptr, block_word_count as u32, false);
        }

        let remainder_byte_count = byte_count & 31;
        let remainder_word_count = remainder_byte_count / size_of::<u32>();

        if remainder_word_count != 0 {
            gba::bios::cpu_set32(
                src_ptr.add(block_word_count),
                dest_ptr.add(block_word_count),
                remainder_word_count as u32,
                false,
            );
        }
    }

    unsafe fn fill_slice_32(dest: &mut [u32], value: u32) {
        let count = dest.len() as u32;
        assert_eq!(count & 31, 0); // 32-byte blocks only
        gba::bios::cpu_fast_set(&value, dest.as_mut_ptr() as *mut u32, count, true);
    }

    unsafe fn fill_slice_16(dest: &mut [u16], value: u16) {
        let count = (dest.len() * size_of::<u16>() / size_of::<u32>()) as u32;
        assert_eq!(count & 31, 0); // 32-byte blocks only
        let src = value as u32 | ((value as u32) << 16);
        gba::bios::cpu_fast_set(&src, dest.as_mut_ptr() as *mut u32, count, true);
    }
}

impl MemoryOps for DMA3 {
    unsafe fn copy_slice_to_address<T: Copy>(src: &[T], dest: usize) {
        let src_ptr = src.as_ptr() as *const u32;
        let dest_ptr = dest as *mut u32;
        let byte_count = src.len() * size_of::<T>();

        if !cfg!(release) {
            // enforce alignment
            assert_eq!(src_ptr as usize & 3, 0);
            assert_eq!(dest_ptr as usize & 3, 0);
            assert_eq!(byte_count & 3, 0);
        }

        let word_count = (byte_count / size_of::<u32>()) as u16;
        Self::set_source(src.as_ptr() as *const u32);
        Self::set_dest(dest as *mut u32);
        Self::set_count(word_count);
        Self::set_control(
            DMAControlSetting::new()
                .with_source_address_control(DMASrcAddressControl::Increment)
                .with_use_32bit(true)
                .with_start_time(DMAStartTiming::Immediate)
                .with_enabled(true),
        );
        asm!("NOP; NOP", options(nomem, nostack));
    }

    unsafe fn fill_slice_32(dest: &mut [u32], value: u32) {
        let count = dest.len() as u16;
        Self::fill32(&value, dest.as_mut_ptr() as *mut u32, count);
    }
}
