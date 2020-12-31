/* Port to Rust/GBA with ARMv4 optimizations by lifning
 *
 * Based on:
 * Simple FLAC decoder (Python)
 *
 * Copyright (c) 2020 Project Nayuki. (MIT License)
 * https://www.nayuki.io/page/simple-flac-implementation
 * 
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 * - The above copyright notice and this permission notice shall be included in
 *   all copies or substantial portions of the Software.
 * - The Software is provided "as is", without warranty of any kind, express or
 *   implied, including but not limited to the warranties of merchantability,
 *   fitness for a particular purpose and noninfringement. In no event shall the
 *   authors or copyright holders be liable for any claim, damages or other
 *   liability, whether in an action of contract, tort or otherwise, arising from,
 *   out of or in connection with the Software or the use or other dealings in the
 *   Software.
 */

use core::mem::size_of;

use crate::audio::PlayableSound;
#[cfg(feature = "verify_asm")] use core::mem::MaybeUninit;
#[cfg(feature = "verify_asm")] use crate::audio::PLAYBUF_SIZE;

// largest would be 12 'cause we're using "Subset" compliant files,
// but we're hardcoding it to 4 in the build script for optimization-path reasons.
type CoefsVec<T> = heapless::Vec<T, heapless::consts::U4>;

pub struct SimpleFlac {
    pub(crate) data: &'static [u32],

    /// encoded data cursor position in data slice
    encoded_position: usize,
    bitbuffer: u32,
    bitbufferlen: usize,

    reset_encoded_position: usize,
    reset_bitbuffer: u32,
    reset_bitbufferlen: usize,

    sample_count: usize,
    sample_depth: u32,
    samples_played: usize,

    looping: bool,
}

impl SimpleFlac {
    pub fn new(data: &'static [u8], looping: bool) -> Self {
        let mut flac = SimpleFlac {
            data: unsafe {
                &*core::ptr::slice_from_raw_parts(data.as_ptr() as *const u32, (data.len() + 3) / 4)
            },
            encoded_position: 0,
            bitbuffer: 0,
            bitbufferlen: 0,
            reset_encoded_position: 0,
            reset_bitbuffer: 0,
            reset_bitbufferlen: 0,
            sample_count: 0,
            sample_depth: 0,
            samples_played: 0,
            looping,
        };
        flac.initialize();
        flac.reset_bitbuffer = flac.bitbuffer;
        flac.reset_bitbufferlen = flac.bitbufferlen;
        flac.reset_encoded_position = flac.encoded_position;
        flac
    }

    #[link_section = ".iwram"]
    fn align_to_byte(&mut self) {
        self.bitbufferlen -= self.bitbufferlen & 7
    }

    #[link_section = ".iwram"]
    fn read_uint(&mut self, mut n: usize) -> u32 {
        #[cfg(feature = "debug_bitbuffer")] warn!("uint before: {:32b}, len {}, n {}", self.bitbuffer, self.bitbufferlen, n);
        if n == 0 {
            return 0;
        }
        let orig_n = n;
        let mut result = 0;
        if self.bitbufferlen == 0 {
            self.replenish_bitbuffer();
        }
        if self.bitbufferlen < n {
            n -= self.bitbufferlen;
            result = self.bitbuffer << n;
            self.bitbufferlen = 0;
            while self.bitbufferlen < n {
                self.replenish_bitbuffer();
            }
            #[cfg(feature = "debug_bitbuffer")] warn!("uint wrap  : {:32b}, len {}, n {}, result 0x{:x}", self.bitbuffer, self.bitbufferlen, n, result);
        }
        self.bitbufferlen -= n;
        result |= self.bitbuffer >> self.bitbufferlen;
        if orig_n < 32 {
            result &= (1 << orig_n) - 1;
        }
        self.bitbuffer &= (1 << self.bitbufferlen) - 1;
        #[cfg(feature = "debug_bitbuffer")] warn!("uint after : {:32b}, len {}, n {}, result 0x{:x}", self.bitbuffer, self.bitbufferlen, n, result);
        result
    }

    #[link_section = ".iwram"]
    fn replenish_bitbuffer(&mut self) {
        if self.encoded_position >= self.data.len() {
            panic!("EOF");
        }
        self.bitbuffer = u32::from_be(unsafe {
            *self.data.get_unchecked(self.encoded_position)
        });
        self.encoded_position += 1;
        self.bitbufferlen += 32;
    }

    #[link_section = ".iwram"]
    fn count_golomb_rice_quotient(&mut self) -> i32 {
        #[cfg(feature = "debug_bitbuffer")]  warn!("rice_naive before: bb {:32b} bbl {} ep {}", self.bitbuffer, self.bitbufferlen, self.encoded_position);
        let mut quotient = 0;
        if self.bitbuffer == 0 {
            quotient = self.bitbufferlen as i32;
            self.bitbufferlen = 0;
        }
        if self.bitbufferlen == 0 {
            self.replenish_bitbuffer();
        }
        while self.bitbuffer & (1 << (self.bitbufferlen - 1)) == 0 {
            quotient += 1;
            self.bitbufferlen -= 1;
            if self.bitbufferlen == 0 {
                 self.replenish_bitbuffer();
            }
        }
        self.bitbufferlen -= 1;
        self.bitbuffer &= (1 << self.bitbufferlen) - 1;
        #[cfg(feature = "debug_bitbuffer")] warn!("rice_naive after:  bb {:32b} bbl {} ep {} q {}", self.bitbuffer, self.bitbufferlen, self.encoded_position, quotient);

        quotient
    }

    #[link_section = ".iwram"]
    fn read_signed_int(&mut self, n: usize) -> i32 {
        let unsigned_value = self.read_uint(n);
        // sign extend
        unsigned_value as i32 - ((unsigned_value >> (n - 1)) << n) as i32
    }

    #[link_section = ".iwram"]
    fn read_rice_signed_int(&mut self, remainder_bits: usize) -> i32 {
        let mut quotient = self.count_golomb_rice_quotient();
        quotient = (quotient << remainder_bits) | self.read_uint(remainder_bits) as i32;
        (quotient >> 1) ^ -(quotient & 1)
    }

    fn initialize(&mut self) {
        // Handle FLAC header and metadata blocks
        let magic = self.read_uint(32);
        // fLaC ascii
        if magic != 0x664C6143 {
            fatal!("Invalid magic: {:x} as {:x}", magic, self.data_ptr() as u32);
        }
        let mut samplerate = 0;
        let mut numchannels = 0;
        let mut last = false;
        while !last {
            last = self.read_uint(1) != 0;
            let type_ = self.read_uint(7);
            let length = self.read_uint(24) as usize;
            // Stream info block
            if type_ == 0 {
                self.read_uint(16 + 16 + 24 + 24);
                samplerate = self.read_uint(20);
                numchannels = self.read_uint(3) + 1;
                self.sample_depth = self.read_uint(5) + 1;
                self.sample_count = self.read_uint(36) as usize;
                self.read_uint(128);
            } else {
                self.read_uint(8 * length);
            }
        }
        if samplerate == 0 {
            panic!("Stream info metadata block absent");
        }
        if self.sample_depth & 7 != 0 {
            fatal!("Sample depth {} not supported", self.sample_depth);
        }
        assert_eq!(numchannels, 1);
    }

    #[link_section = ".iwram"]
    fn decode_frame(&mut self, buf: &mut [i32]) {
        // Read a ton of header fields, and ignore most of them
        let sync = self.read_uint(8 + 6);
        if sync != 0x3FFE {
            fatal!("Sync code expected, got {:x}", sync);
        }

        self.read_uint(1 + 1);
        let blocksizecode = self.read_uint(4);
        let sampleratecode = self.read_uint(4);
        let chanasgn = self.read_uint(4);
        self.read_uint(3 + 1);

        // only supporting monaural
        debug_assert_eq!(chanasgn, 0);

        let mut temp = self.read_uint(8);
        while temp >= 0b11000000 {
            self.read_uint(8);
            temp = (temp << 1) & 0xFF;
        }

        let blocksize = match blocksizecode {
            1 => 192,
            2..=5 => 576 << (blocksizecode - 2),
            6 => self.read_uint(8) + 1,
            7 => self.read_uint(16) + 1,
            8..=15 => 256 << (blocksizecode - 8),
            _ => { fatal!("invalid blocksizecode {}", blocksizecode); },
        } as usize;

        if sampleratecode == 12 {
            self.read_uint(8);
        } else if sampleratecode == 13 || sampleratecode == 14 {
            self.read_uint(16);
        }

        self.read_uint(8);

        // Decode ~~each~~ channel's subframe, then skip footer
        self.decode_subframe(buf, blocksize);
        self.align_to_byte();
        self.read_uint(16);
    }

    #[link_section = ".iwram"]
    fn decode_subframe(&mut self, buf: &mut [i32], blocksize: usize) {
        let mut sampledepth = self.sample_depth as usize;

        self.read_uint(1);
        let type_ = self.read_uint(6) as usize;
        let mut shift = self.read_uint(1) as usize;
        if shift == 1 {
            // technically not a rice int, but same optimization applies
            shift += self.count_golomb_rice_quotient() as usize;
        }
        sampledepth -= shift;

        match type_ {
            0 => self.decode_constant_coding(buf, sampledepth, blocksize),
            1 => self.decode_verbatim_coding(buf, sampledepth, blocksize),
            8..=12 => self.decode_fixed_prediction_subframe(buf, type_ - 8, sampledepth, blocksize),
            32..=63 => self.decode_linear_predictive_coding_subframe(buf, type_ - 31, sampledepth, blocksize),
            _ => { fatal!("Reserved subframe type {}", type_); },
        };
        for x in &mut buf[..blocksize] {
            *x <<= shift;
        }
    }

    #[link_section = ".iwram"]
    fn decode_constant_coding(&mut self, buf: &mut [i32], sampledepth: usize, blocksize: usize) {
        let value = self.read_signed_int(sampledepth);
        let dest = buf.as_mut_ptr() as *mut u32;
        unsafe {
            crate::memory::memset_word(dest, blocksize * size_of::<i32>(), value as u32);
        }
    }

    #[link_section = ".iwram"]
    fn decode_verbatim_coding(&mut self, buf: &mut [i32], sampledepth: usize, blocksize: usize) {
        for x in &mut buf[0..blocksize] {
            *x = self.read_signed_int(sampledepth);
        }
    }

    #[link_section = ".iwram"]
    fn decode_fixed_prediction_subframe(&mut self, buf: &mut [i32], predorder: usize, sampledepth: usize, blocksize: usize) {
        const FIXED_PREDICTION_COEFFICIENTS: [&[i32]; 5] = [
            &[],
            &[1,],
            &[2, -1],
            &[3, -3, 1],
            &[4, -6, 4, -1],
        ];
        #[cfg(feature = "bench_flac")] let start = crate::timers::GbaTimer::get_ticks();
        for x in &mut buf[0..predorder] {
            *x = self.read_signed_int(sampledepth);
        }
        #[cfg(feature = "bench_flac")] let copied = crate::timers::GbaTimer::get_ticks();
        self.decode_residuals(buf, predorder, blocksize);
        #[cfg(feature = "bench_flac")] let decoded = crate::timers::GbaTimer::get_ticks();
        self.restore_linear_prediction(buf, blocksize, FIXED_PREDICTION_COEFFICIENTS[predorder], 0);
        #[cfg(feature = "bench_flac")] let restored = crate::timers::GbaTimer::get_ticks();
        #[cfg(feature = "bench_flac")] info!("fp: {} copy / {} decode / {} restore (ord {})", copied - start, decoded - copied, restored - decoded, predorder);
    }

    #[link_section = ".iwram"]
    fn decode_linear_predictive_coding_subframe(&mut self, buf: &mut [i32], lpcorder: usize, sampledepth: usize, blocksize: usize) {
        #[cfg(feature = "bench_flac")] let start = crate::timers::GbaTimer::get_ticks();
        for x in &mut buf[0..lpcorder] {
            *x = self.read_signed_int(sampledepth);
        }
        let precision = self.read_uint(4) as usize + 1;
        let shift = self.read_signed_int(5);
        let coefs: CoefsVec<i32> = (0..lpcorder).map(|_| self.read_signed_int(precision)).collect();
        #[cfg(feature = "bench_flac")] let copied = crate::timers::GbaTimer::get_ticks();
        self.decode_residuals(buf, lpcorder, blocksize);
        #[cfg(feature = "bench_flac")] let decoded = crate::timers::GbaTimer::get_ticks();
        self.restore_linear_prediction(buf, blocksize, &coefs, shift);
        #[cfg(feature = "bench_flac")] let restored = crate::timers::GbaTimer::get_ticks();
        #[cfg(feature = "bench_flac")] info!("lpc: {} copy / {} decode / {} restore (ord {})", copied - start, decoded - copied, restored - decoded, lpcorder);
    }

    // TODO: possibly hand-optimize somehow???
    #[link_section = ".iwram"]
    fn decode_residuals(&mut self, buf: &mut [i32], mut len: usize, blocksize: usize) {
        let method = self.read_uint(2);
        let (parambits, escapeparam) = match method {
            0 => (4, 0xF),
            1 => (5, 0x1F),
            _ => { fatal!("Reserved residual coding method {}", method); }
        };

        let partitionorder = self.read_uint(4);
        let numpartitions = 1 << partitionorder;
        if blocksize & (numpartitions - 1) != 0 {
            fatal!("Block size {} not divisible by number of Rice partitions {}", blocksize, numpartitions);
        }

        let mut _rice_ticks = 0;
        let mut _reg_ticks = 0;
        for i in 0..numpartitions {
            let start_partition = if cfg!(feature = "bench_flac") { crate::timers::GbaTimer::get_ticks() } else { 0 };

            let mut count = blocksize >> partitionorder;
            if i == 0 {
                count -= len;
            }
            let end = len + count;
            if end > buf.len() {
                fatal!("out of bounds decoding residuals");
            }
            let param = self.read_uint(parambits) as usize;
            if param < escapeparam {
                for x in &mut buf[len..end] {
                    *x = self.read_rice_signed_int(param);
                }
                if cfg!(feature = "bench_flac") { _rice_ticks += crate::timers::GbaTimer::get_ticks() - start_partition; }
            } else {
                let numbits = self.read_uint(5) as usize;
                for x in &mut buf[len..end] {
                    *x = self.read_signed_int(numbits);
                }
                if cfg!(feature = "bench_flac") { _reg_ticks += crate::timers::GbaTimer::get_ticks() - start_partition; }
            }
            len += count;
        }
        #[cfg(feature = "bench_flac")] info!("resid: {} rice (ord {}) / {} reg", _rice_ticks, partitionorder, _reg_ticks);
    }

    //noinspection RsBorrowChecker
    #[link_section = ".iwram"]
    fn restore_linear_prediction(&mut self, buf: &mut [i32], blocksize: usize, coefs: &[i32], shift: i32) {
        let order = coefs.len();
        #[cfg(feature = "bench_flac")]
        debug!("order: {}", order);

        // plain rust version for verification & as reference for what's going on
        #[allow(clippy::uninit_assumed_init)]
        #[cfg(feature = "verify_asm")]
        let verify_buffer = unsafe {
            let mut vbuf: [i32; PLAYBUF_SIZE] = MaybeUninit::uninit().assume_init();
            vbuf.copy_from_slice(&buf);

            for (i, x) in buf[order..blocksize].iter_mut().enumerate() {
                let mut sum = 0;
                for (j, coef) in coefs[0..order].iter().enumerate() {
                    sum += vbuf.get_unchecked(i - 1 - j).unchecked_mul(*coef);
                }
                *x += sum >> shift;
            }

            vbuf
        };

        let mut buf_ptr = unsafe { buf.as_mut_ptr().add(order) };
        let end = unsafe { buf.as_mut_ptr().add(blocksize) };
        // reminders:
        // r6 is used internally by LLVM
        // r11 is the frame pointer, which we evidently can't use even though we're --release
        // r15 is the program counter
        match order {
            0 => {} // lol
            1 => unsafe {
                let coef = *coefs.get_unchecked(0);
                asm!(
                "ldr r8, [{buf}, #-4]", // load previous sample
                "1:",
                "cmp {buf}, {end}",
                "bhi 2f",
                "mul {pred}, r8, {coef}",
                "ldmia {buf}!, {{r0-r5, r7-r8}}", // load next 8 samples
                "add r0, r0, {pred}, asr {shift}",
                "mul {pred}, r0, {coef}",
                "add r1, r1, {pred}, asr {shift}",
                "mul {pred}, r1, {coef}",
                "add r2, r2, {pred}, asr {shift}",
                "mul {pred}, r2, {coef}",
                "add r3, r3, {pred}, asr {shift}",
                "mul {pred}, r3, {coef}",
                "add r4, r4, {pred}, asr {shift}",
                "mul {pred}, r4, {coef}",
                "add r5, r5, {pred}, asr {shift}",
                "mul {pred}, r5, {coef}",
                "add r7, r7, {pred}, asr {shift}",
                "mul {pred}, r7, {coef}",
                "add r8, r8, {pred}, asr {shift}",
                "stmdb {buf}, {{r0-r5, r7-r8}}", // write next 8 samples
                "b 1b",
                "2:",
                buf = inout(reg) buf_ptr,
                end = in(reg) end.sub(8),
                coef = in(reg) coef,
                shift = in(reg) shift,
                pred = out(reg) _,
                out("r0") _,
                out("r1") _,
                out("r2") _,
                out("r3") _,
                out("r4") _,
                out("r5") _,
                out("r7") _,
                out("r8") _,
                options(nostack));
                while buf_ptr < end {
                    let prediction = coef.unchecked_mul(*buf_ptr.sub(1));
                    *buf_ptr += prediction >> shift;
                    buf_ptr = buf_ptr.add(1);
                }
            },
            2 => unsafe {
                asm!(
                "ldmdb {buf}, {{r5, r7}}", // load 2 samples (behind)
                "1:",
                "cmp {buf}, {end}",
                "addhi {end}, #28",
                "bhi 2f",
                "mul {pred}, r5, {coef1}",
                "ldmia {buf}!, {{r0-r5}}", // load 6 samples (ahead) & buf++
                "mla {pred}, r7, {coef0}, {pred}",
                "add r0, r0, {pred}, asr {shift}",
                "mul {pred}, r7, {coef1}",
                "ldr r7, [{buf}], #4", // load 1 more sample (ahead) & buf++
                "mla {pred}, r0, {coef0}, {pred}",
                "add r1, r1, {pred}, asr {shift}",
                "mul {pred}, r0, {coef1}",
                "mla {pred}, r1, {coef0}, {pred}",
                "add r2, r2, {pred}, asr {shift}",
                "mul {pred}, r1, {coef1}",
                "mla {pred}, r2, {coef0}, {pred}",
                "add r3, r3, {pred}, asr {shift}",
                "mul {pred}, r2, {coef1}",
                "mla {pred}, r3, {coef0}, {pred}",
                "add r4, r4, {pred}, asr {shift}",
                "mul {pred}, r3, {coef1}",
                "mla {pred}, r4, {coef0}, {pred}",
                "add r5, r5, {pred}, asr {shift}",
                "mul {pred}, r4, {coef1}",
                "mla {pred}, r5, {coef0}, {pred}",
                "add r7, r7, {pred}, asr {shift}",
                "stmdb {buf}, {{r0-r5, r7}}", // write next 7 samples
                "b 1b",
                "2:", // one-at-a-time loop from here
                "cmp {buf}, {end}",
                "ldrlo r0, [{buf}]", // load 1 sample ahead
                "mullo {pred}, r5, {coef1}",
                "mlalo {pred}, r7, {coef0}, {pred}",
                "addlo r0, r0, {pred}, asr {shift}",
                "strlo r0, [{buf}], #4", // write next sample & buf++
                "movlo r5, r7", // slide sample registers back,
                "movlo r7, r0", // to avoid reloading from mem
                "blo 2b",
                buf = inout(reg) buf_ptr => _,
                end = inout(reg) end.sub(7) => _,
                coef0 = in(reg) *coefs.get_unchecked(0),
                coef1 = in(reg) *coefs.get_unchecked(1),
                shift = in(reg) shift,
                pred = out(reg) _,
                out("r0") _,
                out("r1") _,
                out("r2") _,
                out("r3") _,
                out("r4") _,
                out("r5") _,
                out("r7") _,
                options(nostack));
            }
            3 => unsafe {
                asm!(
                "ldmdb {buf}, {{r3-r5}}", // load 3 samples (behind)
                "1:",
                "cmp {buf}, {end}",
                "addhi {end}, #24",
                "bhi 2f",
                "mul {pred}, r3, {coef2}",
                "ldmia {buf}!, {{r0-r3}}", // load 4 samples (ahead) & buf++
                "mla {pred}, r4, {coef1}, {pred}",
                "mla {pred}, r5, {coef0}, {pred}",
                "add r0, r0, {pred}, asr {shift}",
                "mul {pred}, r4, {coef2}",
                "mla {pred}, r5, {coef1}, {pred}",
                "mla {pred}, r0, {coef0}, {pred}",
                "add r1, r1, {pred}, asr {shift}",
                "mul {pred}, r5, {coef2}",
                "ldmia {buf}!, {{r4-r5}}", // load 2 more samples (ahead) & buf++
                "mla {pred}, r0, {coef1}, {pred}",
                "mla {pred}, r1, {coef0}, {pred}",
                "add r2, r2, {pred}, asr {shift}",
                "mul {pred}, r0, {coef2}",
                "mla {pred}, r1, {coef1}, {pred}",
                "mla {pred}, r2, {coef0}, {pred}",
                "add r3, r3, {pred}, asr {shift}",
                "mul {pred}, r1, {coef2}",
                "mla {pred}, r2, {coef1}, {pred}",
                "mla {pred}, r3, {coef0}, {pred}",
                "add r4, r4, {pred}, asr {shift}",
                "mul {pred}, r2, {coef2}",
                "mla {pred}, r3, {coef1}, {pred}",
                "mla {pred}, r4, {coef0}, {pred}",
                "add r5, r5, {pred}, asr {shift}",
                "stmdb {buf}, {{r0-r5}}", // write next 6 samples.
                "b 1b",
                "2:", // one-at-a-time loop from here
                "cmp {buf}, {end}",
                "ldrlo r0, [{buf}]", // load 1 sample ahead
                "mullo {pred}, r3, {coef2}",
                "mlalo {pred}, r4, {coef1}, {pred}",
                "mlalo {pred}, r5, {coef0}, {pred}",
                "addlo r0, r0, {pred}, asr {shift}",
                "strlo r0, [{buf}], #4", // write next sample & buf++
                "movlo r3, r4", // slide sample registers back,
                "movlo r4, r5", // to avoid reloading from mem
                "movlo r5, r0",
                "blo 2b",
                buf = inout(reg) buf_ptr => _,
                end = inout(reg) end.sub(6) => _,
                coef0 = in(reg) *coefs.get_unchecked(0),
                coef1 = in(reg) *coefs.get_unchecked(1),
                coef2 = in(reg) *coefs.get_unchecked(2),
                shift = in(reg) shift,
                pred = out(reg) _,
                out("r0") _,
                out("r1") _,
                out("r2") _,
                out("r3") _,
                out("r4") _,
                out("r5") _,
                options(nostack));
            }
            4 => unsafe {
                asm!(
                "ldmdb {buf}, {{r1-r4}}", // load 4 samples behind
                "1:",
                "cmp {buf}, {end}",
                "addhi {end}, #20",
                "bhi 2f",
                "mul {pred}, r1, {coef3}",
                "ldmia {buf}!, {{r0-r1}}", // load 2 samples ahead & buf++
                "mla {pred}, r2, {coef2}, {pred}",
                "mla {pred}, r3, {coef1}, {pred}",
                "mla {pred}, r4, {coef0}, {pred}",
                "add r0, r0, {pred}, asr {shift}",
                "mul {pred}, r2, {coef3}",
                "mla {pred}, r3, {coef2}, {pred}",
                "mla {pred}, r4, {coef1}, {pred}",
                "mla {pred}, r0, {coef0}, {pred}",
                "add r1, r1, {pred}, asr {shift}",
                "mul {pred}, r3, {coef3}",
                "ldmia {buf}!, {{r2-r3}}", // load 2 more samples ahead & buf++
                "mla {pred}, r4, {coef2}, {pred}",
                "mla {pred}, r0, {coef1}, {pred}",
                "mla {pred}, r1, {coef0}, {pred}",
                "add r2, r2, {pred}, asr {shift}",
                "mul {pred}, r4, {coef3}",
                "ldr r4, [{buf}], #4", // load 1 more sample ahead & buf++
                "mla {pred}, r0, {coef2}, {pred}",
                "mla {pred}, r1, {coef1}, {pred}",
                "mla {pred}, r2, {coef0}, {pred}",
                "add r3, r3, {pred}, asr {shift}",
                "mul {pred}, r0, {coef3}",
                "mla {pred}, r1, {coef2}, {pred}",
                "mla {pred}, r2, {coef1}, {pred}",
                "mla {pred}, r3, {coef0}, {pred}",
                "add r4, r4, {pred}, asr {shift}",
                "stmdb {buf}, {{r0-r4}}", // write next 5 samples
                "b 1b",
                "2:", // one-at-a-time loop from here
                "cmp {buf}, {end}",
                "ldrlo r0, [{buf}]", // load 1 sample ahead
                "mullo {pred}, r1, {coef3}",
                "mlalo {pred}, r2, {coef2}, {pred}",
                "mlalo {pred}, r3, {coef1}, {pred}",
                "mlalo {pred}, r4, {coef0}, {pred}",
                "addlo r0, r0, {pred}, asr {shift}",
                "strlo r0, [{buf}], #4", // write next sample & buf++
                "movlo r1, r2",
                "movlo r2, r3", // slide sample registers back,
                "movlo r3, r4", // to avoid reloading from mem
                "movlo r4, r0",
                "blo 2b",
                buf = inout(reg) buf_ptr => _,
                end = inout(reg) end.sub(5) => _,
                coef0 = inout(reg) *coefs.get_unchecked(0) => _,
                coef1 = in(reg) *coefs.get_unchecked(1),
                coef2 = in(reg) *coefs.get_unchecked(2),
                coef3 = in(reg) *coefs.get_unchecked(3),
                shift = in(reg) shift,
                pred = out(reg) _,
                out("r0") _,
                out("r1") _,
                out("r2") _,
                out("r3") _,
                out("r4") _,
                options(nostack));
            }
            #[cfg(feature = "flexible_flac")]
            5 => unsafe {
                let mut i = 5;
                while i + 2 < blocksize {
                    asm!(
                    "ldmia {buf}, {{r3-r4}}", // load 2 samples (ahead) - we'll get a third later
                    "mov r2, {buf}", // just so we can ldmdb! below without obliterating buf
                    "ldmdb r2!, {{r0-r1}}", // load 2 samples (behind)
                    "mul {pred}, r0, {coef1}",
                    "mla {pred}, r1, {coef0}, {pred}",
                    "ldmdb r2, {{r0-r2}}", // load 3 samples (farther behind), overwriting r2
                    "mla {pred}, r0, {coef4}, {pred}",
                    "mla {pred}, r1, {coef3}, {pred}",
                    "mla {pred}, r2, {coef2}, {pred}",
                    "add r3, r3, {pred}, asr {shift}",
                    "mul {pred}, r1, {coef4}", // now we're getting fancy... re-use r1!
                    // "mla {pred}, r2, {coef3}, {pred}", <- we could, but we need r2 for 3rd sample
                    "ldmdb {buf}, {{r0-r2}}", // load 3 samples (behind original buf)
                    "mla {pred}, r0, {coef3}, {pred}",
                    "mla {pred}, r1, {coef2}, {pred}",
                    "mla {pred}, r2, {coef1}, {pred}",
                    "mla {pred}, r3, {coef0}, {pred}", // up to our first 'next' sample
                    "add r4, r4, {pred}, asr {shift}",
                    "mul {pred}, r0, {coef4}",
                    "mla {pred}, r1, {coef3}, {pred}",
                    "mla {pred}, r2, {coef2}, {pred}",
                    "mla {pred}, r3, {coef1}, {pred}",
                    "mla {pred}, r4, {coef0}, {pred}",
                    // TODO: possible unroll further from here, less-awkwardly since we have all the
                    //  registers computed?  like do a stmia, then load one r# at a time?
                    //  instead of coef0 obv
                    "ldr {coef0}, [{buf}, #8]", // load third-ahead sample... reusing coef0.
                    "add {coef0}, {coef0}, {pred}, asr {shift}",
                    "stmia {buf}, {{r3-r4, {coef0}}}", // write next 3 samples... (stmia order is ok
                    // we're already occupying r0-r4, so we can assume that coef0 can't be lower)
                    buf = in(reg) buf.as_ptr().add(i),
                    coef0 = inout(reg) *coefs.get_unchecked(0) => _,
                    coef1 = in(reg) *coefs.get_unchecked(1),
                    coef2 = in(reg) *coefs.get_unchecked(2),
                    coef3 = in(reg) *coefs.get_unchecked(3),
                    coef4 = in(reg) *coefs.get_unchecked(4),
                    shift = in(reg) shift,
                    pred = out(reg) _,
                    out("r0") _,
                    out("r1") _,
                    out("r2") _,
                    out("r3") _,
                    out("r4") _,
                    options(nostack));
                    i += 3;
                }
                while i < blocksize {
                    asm!(
                    "mov r2, {buf}", // just so we can ldmdb! below without obliterating buf
                    "ldmda r2!, {{r0-r1, r3}}", // load 2 samples behind (r0-r1) and one ahead (r3)
                    "ldmia {coefs}!, {{r7-r8}}", // load next 2 coefs
                    "mul {pred}, r0, r8",
                    "mla {pred}, r1, r7, {pred}",
                    "ldmda r2, {{r0-r2}}", // load 3 samples farther back, overwriting r2
                    "ldmia {coefs}, {{r7-r9}}", // load last 3 coefs
                    "mla {pred}, r0, r9, {pred}",
                    "mla {pred}, r1, r8, {pred}",
                    "mla {pred}, r2, r7, {pred}",
                    "add r3, r3, {pred}, asr {shift}",
                    "str r3, [{buf}]", // write next sample
                    buf = in(reg) buf.as_ptr().add(i),
                    coefs = inout(reg) coefs.as_ptr() => _,
                    shift = in(reg) shift,
                    pred = out(reg) _,
                    out("r0") _,
                    out("r1") _,
                    out("r2") _,
                    out("r3") _,
                    out("r7") _,
                    out("r8") _,
                    out("r9") _,
                    options(nostack));
                    i += 1;
                }
            }
            #[cfg(feature = "flexible_flac")]
            6 => unsafe {
                let mut prediction;
                let mut i = 6;
                while i < blocksize {
                    asm!(
                    "ldmdb {0}!, {{r1-r3}}", // load previous 3 samples
                    "ldmia {1}!, {{r7-r9}}", // load next 3 coefs
                    "mul r0, r1, r9",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "ldmdb {0}, {{r1-r3}}", // load previous 3 samples
                    "ldmia {1}, {{r7-r9}}", // load last 3 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    inout(reg) buf.as_ptr().add(i) => _,
                    inout(reg) coefs.as_ptr() => _,
                    out("r0") prediction,
                    out("r1") _,
                    out("r2") _,
                    out("r3") _,
                    lateout("r7") _,
                    lateout("r8") _,
                    lateout("r9") _,
                    options(nostack));
                    *buf.get_unchecked_mut(i) += prediction >> shift;
                    i += 1;
                }
            }
            #[cfg(feature = "flexible_flac")]
            7 => unsafe {
                let mut prediction;
                for i in 7..blocksize {
                    asm!(
                    "ldmdb {0}!, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}!, {{r5, r7-r9}}", // load next 4 coefs
                    "mul r0, r1, r9",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    "ldmdb {0}, {{r1-r3}}", // load previous 3 samples
                    "ldmia {1}, {{r7-r9}}", // load last 3 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    inout(reg) buf.as_ptr().add(i) => _,
                    inout(reg) coefs.as_ptr() => _,
                    out("r0") prediction,
                    out("r1") _,
                    out("r2") _,
                    out("r3") _,
                    out("r4") _,
                    lateout("r5") _,
                    lateout("r7") _,
                    lateout("r8") _,
                    lateout("r9") _,
                    options(nostack));
                    *buf.get_unchecked_mut(i) += prediction >> shift;
                }
            }
            #[cfg(feature = "flexible_flac")]
            8 => unsafe {
                let mut prediction;
                for i in 8..blocksize {
                    asm!(
                    "ldmdb {0}!, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}!, {{r5, r7-r9}}", // load next 4 coefs
                    "mul r0, r1, r9",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    "ldmdb {0}, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}, {{r5, r7-r9}}", // load last 4 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    inout(reg) buf.as_ptr().add(i) => _,
                    inout(reg) coefs.as_ptr() => _,
                    out("r0") prediction,
                    out("r1") _,
                    out("r2") _,
                    out("r3") _,
                    out("r4") _,
                    lateout("r5") _,
                    lateout("r7") _,
                    lateout("r8") _,
                    lateout("r9") _,
                    options(nostack));
                    *buf.get_unchecked_mut(i) += prediction >> shift;
                }
            }
            #[cfg(feature = "flexible_flac")]
            9 => unsafe {
                let mut prediction;
                for i in 9..blocksize {
                    asm!(
                    "ldmdb {0}!, {{r1-r3}}", // load previous 3 samples
                    "ldmia {1}!, {{r7-r9}}", // load next 3 coefs
                    "mul r0, r1, r9",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "ldmdb {0}!, {{r1-r3}}", // load previous 3 samples
                    "ldmia {1}!, {{r7-r9}}", // load next 3 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "ldmdb {0}, {{r1-r3}}", // load previous 3 samples
                    "ldmia {1}, {{r7-r9}}", // load next (last) 3 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    inout(reg) buf.as_ptr().add(i) => _,
                    inout(reg) coefs.as_ptr() => _,
                    out("r0") prediction,
                    out("r1") _,
                    out("r2") _,
                    out("r3") _,
                    lateout("r7") _,
                    lateout("r8") _,
                    lateout("r9") _,
                    options(nostack));
                    *buf.get_unchecked_mut(i) += prediction >> shift;
                }
            }
            #[cfg(feature = "flexible_flac")]
            10 => for i in 10..blocksize {
                let mut prediction;
                unsafe {
                    asm!(
                    "ldmdb {0}!, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}!, {{r5, r7-r9}}", // load next 4 coefs
                    "mul r0, r1, r9",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    "ldmdb {0}!, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}!, {{r5, r7-r9}}", // load next 4 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    "ldmdb {0}, {{r1-r2}}", // load previous 2 samples
                    "ldmia {1}, {{r8-r9}}", // load last 2 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    inout(reg) buf.as_ptr().add(i) => _,
                    inout(reg) coefs.as_ptr() => _,
                    out("r0") prediction,
                    out("r1") _,
                    out("r2") _,
                    out("r3") _,
                    out("r4") _,
                    lateout("r5") _,
                    lateout("r7") _,
                    lateout("r8") _,
                    lateout("r9") _,
                    options(nostack));
                    *buf.get_unchecked_mut(i) += prediction >> shift;
                }
            }
            #[cfg(feature = "flexible_flac")]
            11 => for i in 11..blocksize {
                let mut prediction;
                unsafe {
                    asm!(
                    "ldmdb {0}!, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}!, {{r5, r7-r9}}", // load next 4 coefs
                    "mul r0, r1, r9",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    "ldmdb {0}!, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}!, {{r5, r7-r9}}", // load next 4 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    "ldmdb {0}, {{r1-r3}}", // load previous 3 samples
                    "ldmia {1}, {{r7-r9}}", // load last 3 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    inout(reg) buf.as_ptr().add(i) => _,
                    inout(reg) coefs.as_ptr() => _,
                    out("r0") prediction,
                    out("r1") _,
                    out("r2") _,
                    out("r3") _,
                    out("r4") _,
                    lateout("r5") _,
                    lateout("r7") _,
                    lateout("r8") _,
                    lateout("r9") _,
                    options(nostack));
                    *buf.get_unchecked_mut(i) += prediction >> shift;
                }
            }
            #[cfg(feature = "flexible_flac")]
            12 => for i in 12..blocksize {
                let mut prediction;
                unsafe {
                    asm!(
                    "ldmdb {0}!, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}!, {{r5, r7-r9}}", // load next 4 coefs
                    "mul r0, r1, r9",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    "ldmdb {0}!, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}!, {{r5, r7-r9}}", // load next 4 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    "ldmdb {0}, {{r1-r4}}", // load previous 4 samples
                    "ldmia {1}, {{r5, r7-r9}}", // load last 4 coefs
                    "mla r0, r1, r9, r0",
                    "mla r0, r2, r8, r0",
                    "mla r0, r3, r7, r0",
                    "mla r0, r4, r5, r0",
                    inout(reg) buf.as_ptr().add(i) => _,
                    inout(reg) coefs.as_ptr() => _,
                    out("r0") prediction,
                    out("r1") _,
                    out("r2") _,
                    out("r3") _,
                    out("r4") _,
                    lateout("r5") _,
                    lateout("r7") _,
                    lateout("r8") _,
                    lateout("r9") _,
                    options(nostack));
                    *buf.get_unchecked_mut(i) += prediction >> shift;
                }
            }
            x => { fatal!("Too many coefficients! {}", x); }
        }

        #[cfg(feature = "verify_asm")]
        {
            let mut mismatches = 0;
            for (i, (&asm, &rust)) in buf.iter().zip(verify_buffer.iter()).enumerate() {
                if asm != rust {
                    mismatches += 1;
                    if mismatches > 1 {
                        warn!("buf[{}] {:x} != {:x}", i, asm, rust);
                    }
                }
            }
            if mismatches > 1 {
                fatal!("restore_linear, coefs {:?}, {} / {}", coefs, mismatches, verify_buffer.len());
            }
        }
    }
}

impl PlayableSound for SimpleFlac {
    #[link_section = ".iwram"]
    fn mix_into(&mut self, mixbuf: &mut [i32]) {
        if !self.finished() {
            if self.samples_played >= self.sample_count && self.looping() {
                debug!("Resetting playback!");
                self.reset();
            }
            // HACK: doesn't "mix", overwrites mixbuf entirely, but BGM always first in line so w/e
            self.decode_frame(mixbuf);
            self.samples_played += mixbuf.len();
            debug!("samples played: {} count: {}", self.samples_played, self.sample_count);

            // normalize sample depth to 16-bit
            #[cfg(feature = "flexible_flac")]
            {
                let leftshift = 16 - self.sample_depth as i32;
                if leftshift > 0 {
                    debug_assert_eq!(mixbuf.len() & 7, 0);
                    for i in 0..(mixbuf.len() as isize / 8) {
                        unsafe {
                            asm!(
                            "ldmia {mixbuf}, {{r0-r5, r7-r8}}", // load eight 32-bit samples
                            "mov r0, r0, lsl {leftshift}",
                            "mov r1, r1, lsl {leftshift}",
                            "mov r2, r2, lsl {leftshift}",
                            "mov r3, r3, lsl {leftshift}",
                            "mov r4, r4, lsl {leftshift}",
                            "mov r5, r5, lsl {leftshift}",
                            "mov r7, r7, lsl {leftshift}",
                            "mov r8, r8, lsl {leftshift}",
                            "stmia {mixbuf}, {{r0-r5, r7-r8}}", // write eight 32-bit samples
                            mixbuf = in(reg) mixbuf.as_ptr().offset(i * 8),
                            leftshift = in(reg) leftshift,
                            out("r0") _,
                            out("r1") _,
                            out("r2") _,
                            out("r3") _,
                            out("r4") _,
                            out("r5") _,
                            out("r7") _,
                            out("r8") _,
                            options(nostack));
                        }
                    }
                } else if leftshift < 0 {
                    debug_assert_eq!(mixbuf.len() & 7, 0);
                    for i in 0..(mixbuf.len() as isize / 8) {
                        unsafe {
                            asm!(
                            "ldmia {mixbuf}, {{r0-r5, r7-r8}}", // load eight 32-bit samples
                            "mov r0, r0, lsr {rightshift}",
                            "mov r1, r1, lsr {rightshift}",
                            "mov r2, r2, lsr {rightshift}",
                            "mov r3, r3, lsr {rightshift}",
                            "mov r4, r4, lsr {rightshift}",
                            "mov r5, r5, lsr {rightshift}",
                            "mov r7, r7, lsr {rightshift}",
                            "mov r8, r8, lsr {rightshift}",
                            "stmia {mixbuf}, {{r0-r5, r7-r8}}", // write eight 32-bit samples
                            mixbuf = in(reg) mixbuf.as_ptr().offset(i * 8),
                            rightshift = in(reg) -leftshift,
                            out("r0") _,
                            out("r1") _,
                            out("r2") _,
                            out("r3") _,
                            out("r4") _,
                            out("r5") _,
                            out("r7") _,
                            out("r8") _,
                            options(nostack));
                        }
                    }
                }
            }
        }
    }

    fn remaining_samples(&self) -> usize {
        if self.samples_played > self.sample_count {
            0
        } else {
            self.sample_count - self.samples_played
        }
    }

    fn looping(&self) -> bool {
        self.looping
    }

    fn data_ptr(&self) -> *const u8 {
        self.data.as_ptr() as *const u8
    }

    fn reset(&mut self) {
        self.bitbuffer = self.reset_bitbuffer;
        self.bitbufferlen = self.reset_bitbufferlen;
        self.encoded_position = self.reset_encoded_position;
        self.samples_played = 0;
    }
}
