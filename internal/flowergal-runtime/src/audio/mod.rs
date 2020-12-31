use core::ops::{Deref, DerefMut};

use gba::io::dma::{
    DMAControlSetting, DMADestAddressControl, DMASrcAddressControl, DMAStartTiming, DMA1, DMA2,
};
use gba::io::sound::{
    NumberSoundVolume, SoundMasterSetting, WaveVolumeEnableSetting, FIFO_A_L, FIFO_B_L, SOUNDBIAS,
    SOUNDCNT_H, SOUNDCNT_X,
};
use gba::io::timers::{TimerControlSetting, TimerTickRate, TM0CNT_H, TM0CNT_L};
#[cfg(not(feature = "supercard"))]
use gba::rom::{WaitstateControl, WaitstateFirstAccess, WAITCNT};

use heapless::Vec;

use crate::audio::raw_pcm::RawPcm8;
use flowergal_proj_config::resources::Sound;
use flowergal_proj_config::sound_info::{PLAYBUF_SIZE, TIMER_VALUE};
use crate::audio::simple_flac::SimpleFlac;

pub mod raw_pcm;
pub mod simple_flac;

type NumChannels = heapless::consts::U4;

#[repr(align(4))]
struct PlayBuffer(pub [i8; PLAYBUF_SIZE]);

pub trait PlayableSound {
    fn mix_into(&mut self, mixbuf: &mut [i32]);
    fn remaining_samples(&self) -> usize;
    fn looping(&self) -> bool;
    fn data_ptr(&self) -> *const u8;
    fn reset(&mut self);

    fn finished(&self) -> bool {
        self.remaining_samples() == 0 && !self.looping()
    }
}

pub enum RuntimeSoundData {
    RawPcm8(RawPcm8),
    Flac(SimpleFlac),
}

impl From<&Sound> for RuntimeSoundData {
    fn from(s: &Sound) -> Self {
        // FIXME: assumes FLAC is looping and raw PCM is not
        assert_eq!(s.data_ptr() as usize & 3, 0);
        match s {
            Sound::RawPcm8(data) => RuntimeSoundData::RawPcm8(RawPcm8::new(data, false)),
            Sound::Flac(data) => RuntimeSoundData::Flac(SimpleFlac::new(data, true)),
        }
    }
}

impl Deref for RuntimeSoundData {
    type Target = dyn PlayableSound;

    fn deref(&self) -> &Self::Target {
        match self {
            RuntimeSoundData::RawPcm8(x) => x,
            RuntimeSoundData::Flac(x) => x,
        }
    }
}

impl DerefMut for RuntimeSoundData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            RuntimeSoundData::RawPcm8(x) => x,
            RuntimeSoundData::Flac(x) => x,
        }
    }
}

pub struct AudioDriver {
    playbuf_a: [PlayBuffer; 2],
    playbuf_b: [PlayBuffer; 2],
    cur_playbuf: usize,
    cur_bgm: Option<usize>,
    #[cfg(feature = "detect_silence")]
    is_silenced: bool,
    #[cfg(feature = "detect_silence")]
    should_silence: bool,
    sounds: Vec<RuntimeSoundData, NumChannels>,
    pub ticks_decode: u32,
    pub ticks_unmix: u32,
}

const fn buf_a_to_b_distance() -> usize {
    //const driver: AudioDriver = AudioDriver::new();
    const DISTANCE: usize = core::mem::size_of::<PlayBuffer>() * 2;
    /*
    const distance = unsafe {
        driver.playbuf_b.as_ptr() as usize - driver.playbuf_a.as_ptr() as usize
    };
     */
    /*
    unsafe {
        const _after_offset: *const i8 = driver.playbuf_a[0].0.as_ptr().offset(distance as isize);
        const _b_ptr: *const i8 = driver.playbuf_b[0].0.as_ptr();
        const_assert_eq!(_after_offset, _b_ptr);
    }
    */
    //core::mem::forget(driver);
    DISTANCE
}

#[cfg(feature = "detect_silence")]
const SILENCE_DETECT_THRESHOLD_MASK: u32 = 0xfefefefe;

impl AudioDriver {
    pub const fn new() -> Self {
        AudioDriver {
            playbuf_a: [PlayBuffer([0; PLAYBUF_SIZE]), PlayBuffer([0; PLAYBUF_SIZE])],
            playbuf_b: [PlayBuffer([0; PLAYBUF_SIZE]), PlayBuffer([0; PLAYBUF_SIZE])],

            cur_playbuf: 0,
            cur_bgm: None,

            #[cfg(feature = "detect_silence")]
            is_silenced: false,
            #[cfg(feature = "detect_silence")]
            should_silence: false,

            sounds: Vec(heapless::i::Vec::new()),
            ticks_decode: 0,
            ticks_unmix: 0
        }
    }

    pub fn initialize(&self) {
        // proper ROM access times important to timely audio stream decoding!
        #[cfg(not(feature = "supercard"))]
        WAITCNT.write(
            WaitstateControl::new()
                .with_sram(WaitstateFirstAccess::Cycles8)
                .with_ws0_first_access(WaitstateFirstAccess::Cycles3)
                .with_ws0_second_access(true)
                .with_ws2_first_access(WaitstateFirstAccess::Cycles8)
                .with_game_pak_prefetch_buffer(true),
        );

        TM0CNT_H.write(TimerControlSetting::new());

        unsafe {
            DMA1::set_dest(FIFO_A_L.to_usize() as *mut u32);
            DMA1::set_count(1);
        }

        unsafe {
            DMA2::set_dest(FIFO_B_L.to_usize() as *mut u32);
            DMA2::set_count(1);
        }

        // turn on sound circuit
        SOUNDCNT_X.write(SoundMasterSetting::new().with_psg_fifo_master_enabled(true));

        // full volume, enable both directsound channels to left and right
        SOUNDCNT_H.write(
            WaveVolumeEnableSetting::new()
                .with_sound_number_volume(NumberSoundVolume::Full)
                .with_dma_sound_a_full_volume(true)
                .with_dma_sound_a_enable_right(true)
                .with_dma_sound_a_enable_left(true)
                .with_dma_sound_a_reset_fifo(true)
                .with_dma_sound_a_timer_select(false) // 0
                .with_dma_sound_b_full_volume(true)
                .with_dma_sound_b_enable_right(true)
                .with_dma_sound_b_enable_left(true)
                .with_dma_sound_b_reset_fifo(true)
                .with_dma_sound_b_timer_select(false), // 0
        );
        TM0CNT_L.write(TIMER_VALUE);
        TM0CNT_H.write(
            TimerControlSetting::new()
                .with_tick_rate(TimerTickRate::CPU1)
                .with_enabled(true),
        );

        gba::bios::sound_bias(0x200);
        SOUNDBIAS.write(SOUNDBIAS.read().with_amplitude_resolution(0));
    }

    // for crash handler
    pub fn disable(&self) {
        gba::bios::sound_bias(0x0);
        gba::bios::sound_channel_clear();

        // turn on sound circuit
        SOUNDCNT_X.write(SoundMasterSetting::new().with_psg_fifo_master_enabled(false));

        // full volume, enable both directsound channels to left and right
        SOUNDCNT_H.write(
            WaveVolumeEnableSetting::new()
                .with_sound_number_volume(NumberSoundVolume::Quarter)
                .with_dma_sound_a_full_volume(false)
                .with_dma_sound_a_enable_right(false)
                .with_dma_sound_a_enable_left(false)
                .with_dma_sound_a_reset_fifo(true)
                .with_dma_sound_a_timer_select(false) // 0
                .with_dma_sound_b_full_volume(false)
                .with_dma_sound_b_enable_right(false)
                .with_dma_sound_b_enable_left(false)
                .with_dma_sound_b_reset_fifo(true)
                .with_dma_sound_b_timer_select(false), // 0
        );
    }

    fn remove_stale_sound(&mut self) {
        if let Some((index, _)) = self
            .sounds
            .iter()
            .enumerate()
            .filter(|(_, rt_sound)| !rt_sound.looping())
            .min_by_key(|(_, rt_sound)| rt_sound.remaining_samples())
        {
            self.remove_sound(index);
        }
    }

    fn remove_sound(&mut self, index: usize) {
        self.sounds.swap_remove(index);
        if let Some(x) = self.cur_bgm.as_mut() {
            if *x == self.sounds.len() {
                *x = index;
            }
        }
    }

    pub fn set_bgm(&mut self, sound: &Sound) {
        if let Some(index) = self.cur_bgm {
            if unsafe { self.sounds.get_unchecked(index) }.data_ptr() == sound.data_ptr() {
                return;
            }
            self.sounds.swap_remove(index);
            self.cur_bgm = None;
        }

        if self.sounds.len() == self.sounds.capacity() {
            self.remove_stale_sound();
        }
        let rt_sound = RuntimeSoundData::from(sound);
        if let Err(..) = self.sounds.push(rt_sound) {
            error!("mixer has no room for bgm at {:?}", sound.data_ptr());
        }
        self.cur_bgm = Some(self.sounds.len() - 1);
    }

    pub fn play_sfx(&mut self, sound: &Sound) {
        if self.sounds.len() == self.sounds.capacity() {
            self.remove_stale_sound();
        }
        let rt_sound = RuntimeSoundData::from(sound);
        if let Err(..) = self.sounds.push(rt_sound) {
            error!("mixer has no room for sfx at {:?}", sound.data_ptr());
        }
    }

    /// Timing-sensitive - call this immediately upon entering VBlank ISR!
    #[link_section = ".iwram"]
    #[instruction_set(arm::a32)]
    pub fn dsound_vblank(&mut self) {
        let (src_a, src_b) = self.cur_playbufs();
        unsafe {
            DMA1::set_control(DMAControlSetting::new());
            DMA2::set_control(DMAControlSetting::new());

            // no-op to let DMA registers catch up
            asm!("NOP; NOP", options(nomem, nostack));

            DMA1::set_source(src_a.as_ptr() as *const u32);
            DMA2::set_source(src_b.as_ptr() as *const u32);

            const DMA_CONTROL_FLAGS: DMAControlSetting = DMAControlSetting::new()
                .with_dest_address_control(DMADestAddressControl::Fixed)
                .with_source_address_control(DMASrcAddressControl::Increment)
                .with_dma_repeat(true)
                .with_use_32bit(true)
                .with_start_time(DMAStartTiming::Special)
                .with_enabled(true);

            DMA1::set_control(DMA_CONTROL_FLAGS);
            DMA2::set_control(DMA_CONTROL_FLAGS);
        }
        self.cur_playbuf = 1 - self.cur_playbuf;

        #[cfg(feature = "detect_silence")]
        if !self.is_silenced && self.should_silence {
            self.is_silenced = true;
            gba::bios::sound_bias(0x0);
        }
    }

    fn cur_playbufs(&mut self) -> (&mut [i8; PLAYBUF_SIZE], &mut [i8; PLAYBUF_SIZE]) {
        unsafe {
            (
                &mut self.playbuf_a.get_unchecked_mut(self.cur_playbuf).0,
                &mut self.playbuf_b.get_unchecked_mut(self.cur_playbuf).0,
            )
        }
    }

    pub fn prev_playbufs(&self) -> (&[i8; PLAYBUF_SIZE], &[i8; PLAYBUF_SIZE]) {
        unsafe {
            (
                &self.playbuf_a.get_unchecked(1 - self.cur_playbuf).0,
                &self.playbuf_b.get_unchecked(1 - self.cur_playbuf).0,
            )
        }
    }

    //noinspection RsBorrowChecker (can't tell that we've written to silence_detect_tmp)
    /// Call this once per frame, at some point after dsound_vblank().
    #[link_section = ".iwram"]
    pub fn mixer(&mut self) {
        let start = super::timers::GbaTimer::get_ticks();

        let mut mix_buffer = [0i32; PLAYBUF_SIZE];
        for sound in self.sounds.iter_mut() {
            sound.mix_into(&mut mix_buffer);
        }

        let decoded = super::timers::GbaTimer::get_ticks();

        // split into two channels.  not for stereo reasons, but so we can get a cheeky 9th bit of
        // audio quality out of the gba's typically 8-bit sound registers, by rounding up in one of
        // the two channels for samples where it's relevant.  the gba will wiggle its PWM at the
        // amplitude a + b.  (proving that we get our 9th bit back as a result is an easy exercise)
        let (buf_a, _buf_b) = self.cur_playbufs();
        #[cfg(feature = "detect_silence")]
        let mut silence_detect = 0u32;
        for i in 0..(mix_buffer.len() as isize / 8) {
            let mut _silence_detect_tmp: u32;
            unsafe {
                asm!(
                "ldmia r9, {{r0-r5, r7-r8}}", // load eight 32-bit samples from mixbuf
                // initializing the ninth-bit register with the 2nd sample first for shifty reasons
                "ands r9, r1, #0x0080", // grab ninth-bit of second sample
                "movne r9, r9, lsl #1", // reposition it if it's there
                "and r1, r1, #0xff00", // mask it & sign bits off
                // 1st sample
                "movs r0, r0, ror #8", // ninth-bit becomes sign-bit
                "orrmi r9, 0x01", // ninth-bit for first sample
                "and r0, r0, #0xff", // clear sign bits
                "orr r0, r0, r1", // merge in conveniently-positioned second sample
                // 3rd sample
                "movs r2, r2, ror #8", // ninth-bit becomes sign-bit
                "orrmi r9, 0x010000", // ninth-bit for 3rd sample (remember, little endian)
                "and r2, r2, #0xff", // clear sign bits
                "orr r0, r0, r2, lsl #16",
                // 4th sample
                "movs r3, r3, ror #8", // ninth-bit becomes sign-bit
                "orrmi r9, 0x01000000", // ninth-bit for 4th sample (remember, little endian)
                "and r3, r3, #0xff", // clear sign bits
                "orr r0, r0, r3, lsl #24",
                // playbuf b's copy with the ninth-bits added
                "add r1, r0, r9",

                // as above for the second set of four samples:
                // initializing the ninth-bit register with the 2nd sample first for shifty reasons
                "ands r9, r5, #0x0080", // grab ninth-bit of second sample
                "movne r9, r9, lsl #1", // reposition it if it's there
                "and r5, r5, #0xff00", // mask it & sign bits off
                // 1st sample
                "movs r4, r4, ror #8", // ninth-bit becomes sign-bit
                "orrmi r9, 0x01", // ninth-bit for first sample
                "and r4, r4, #0xff", // clear sign bits
                "orr r4, r4, r5", // merge in conveniently-positioned second sample
                // 3rd sample
                "movs r7, r7, ror #8", // ninth-bit becomes sign-bit
                "orrmi r9, 0x010000", // ninth-bit for 3rd sample (remember, little endian)
                "and r7, r7, #0xff", // clear sign bits
                "orr r4, r4, r7, lsl #16",
                // 4th sample
                "movs r8, r8, ror #8", // ninth-bit becomes sign-bit
                "orrmi r9, 0x01000000", // ninth-bit for 4th sample (remember, little endian)
                "and r8, r8, #0xff", // clear sign bits
                "orr r4, r4, r8, lsl #24",
                // playbuf b's copy with the ninth-bits added
                "add r5, r4, r9",

                // detect silence by seeing if any bits were set at all in the output buffer
                "orr r9, r0, r4",
                "stmia {buf_a}, {{r0, r4}}", // write eight 8-bit samples to buf_a
                "add {buf_a}, {buf_a}, {BUF_A_TO_B_DISTANCE}",
                "stmia {buf_a}, {{r1, r5}}", // write eight 8-bit samples to buf_b
                buf_a = in(reg) buf_a.as_ptr().offset(i * 8),
                // we save a register here by adding size_of::<Playbuf>() * 2 to buf_a to get buf_b
                BUF_A_TO_B_DISTANCE = const buf_a_to_b_distance(),
                // NOTE: once we're done loading, we immediately start reusing the 'mixbuf' register
                // for 9th-bit scratch space, and after that we use it to detect silence.
                inout("r9") mix_buffer.as_ptr().offset(i * 8) => _silence_detect_tmp,
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

            #[cfg(feature = "detect_silence")]
            {
                silence_detect |= _silence_detect_tmp;
            }
        }

        #[cfg(feature = "detect_silence")]
        {
            self.should_silence = silence_detect & SILENCE_DETECT_THRESHOLD_MASK == 0;
            if !self.should_silence && self.is_silenced {
                gba::bios::sound_bias(0x200);
                self.is_silenced = false;
            }
        }

        #[cfg(feature = "verify_asm")]
        {
            let (mut ref_a, mut ref_b) = ([0i8; PLAYBUF_SIZE], [0i8; PLAYBUF_SIZE]);
            for ((a, b), mixed) in ref_a
                .iter_mut()
                .zip(ref_b.iter_mut())
                .zip(mix_buffer.iter())
            {
                let val = (mixed >> 8).clamp(-128, 127) as i8;
                *a = val;
                *b = val + if mixed & 0x0080 != 0 { 1 } else { 0 };
            }

            let mut mismatches = 0;
            for (mix, ((a1, a2), (b1, b2))) in mix_buffer.iter().zip(buf_a.iter().zip(ref_a.iter()).zip(_buf_b.iter().zip(ref_b.iter()))) {
                if *a1 != *a2 {
                    mismatches += 1;
                    if mismatches > 8 {
                        warn!("{:x} | {:x}={:x} | {:x}={:x}", mix, a1, a2, b1, b2);
                    }
                }
            }
            if mismatches > 8 {
                panic!();
            }
        }

        let split = super::timers::GbaTimer::get_ticks();

        self.ticks_unmix = split - decoded;
        self.ticks_decode = decoded - start;

        let mut index = 0;
        // while loop because length changes during iteration
        while index < self.sounds.len() {
            // only increment in else because swap_remove swaps with end of vec, which we should also check
            if unsafe { self.sounds.get_unchecked(index) }.finished() {
                self.remove_sound(index);
            } else {
                index += 1;
            }
        }

        #[cfg(feature = "bench_audio")] let finish = super::timers::GbaTimer::get_ticks();
        #[cfg(feature = "bench_audio")] info!("{} decode / {} split / {} finish / tick {}", decoded - start, split - decoded, finish - split, finish);

        // (be sure to multiply by the tick rate divisor used in timers.rs)
        // baseline
        // - 1 adpcm: 30,000 cycles decode / 20,000 cycles split / 156 cycles finish
        // - 1 adpcm + 3 pcm: 55,000 cycles decode
        // switched entire project from thumbv4t to armv4t:
        // - 1 adpcm: 21,000 cycles decode / 21,000 cycles split / 188 finish
        // - 1 adpcm + 3 pcm: 46,000 decode / 21,000 split / 674 finish
        // removed clamp from split: 12,000 cycle split
        // (potential for 8,000 cycle (or better) split if we forget about the 9-bit trickery)
    }
}
