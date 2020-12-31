use crate::audio::PlayableSound;

pub struct RawPcm8 {
    pub(crate) data: &'static [u8],
    /// decode position in 8-bit samples
    decode_position: usize,
    sample_count: usize,
    looping: bool,
}

impl RawPcm8 {
    pub const fn new(data: &'static [u8], looping: bool) -> Self {
        RawPcm8 {
            data,
            decode_position: 0,
            sample_count: data.len(),
            looping,
        }
    }
}

impl PlayableSound for RawPcm8 {
    #[link_section = ".iwram"]
    fn mix_into(&mut self, mixbuf: &mut [i32]) {
        let mut remaining = self.remaining_samples();
        if remaining == 0 {
            if self.looping() {
                self.reset();
                remaining = self.remaining_samples();
            } else {
                return;
            }
        }
        let to_decode = mixbuf.len().min(remaining);
        for i in 0..(to_decode / 8) {
            unsafe {
                asm!(
                "ldmia r12, {{r0-r1}}", // load eight 8-bit samples
                "ldmia {mix}, {{r2-r5, r7-r10}}", // mixing with eight 32-bit samples
                "and r12, r0, #0xff",
                "add r2, r2, r12, lsl #8",
                "and r12, r0, #0xff00",
                "add r3, r3, r12",
                "and r12, r0, #0xff0000",
                "add r4, r4, r12, lsr #8",
                "and r12, r0, #0xff000000",
                "add r5, r5, r12, lsr #16",
                "and r12, r1, #0xff",
                "add r7, r7, r12, lsl #8",
                "and r12, r1, #0xff00",
                "add r8, r8, r12",
                "and r12, r1, #0xff0000",
                "add r9, r9, r12, lsr #8",
                "and r12, r1, #0xff000000",
                "add r10, r10, r12, lsr #16",
                "stmia {mix}, {{r2-r5, r7-r10}}", // write back eight 32-bit samples
                mix = inout(reg) mixbuf.as_ptr().add(i * 8) => _,
                // we reuse r12 for scratch space
                inout("r12") self.data.as_ptr().add(i * 8 + self.decode_position) => _,
                out("r0") _,
                out("r1") _,
                out("r2") _,
                out("r3") _,
                out("r4") _,
                out("r5") _,
                out("r7") _,
                out("r8") _,
                out("r9") _,
                out("r10") _,
                options(nostack));
            }
        }
        self.decode_position += to_decode;
        /*
        let len = mixbuf.len().min(remaining);
        for mb in mixbuf[0..len].iter_mut() {
            *mb += self.decode_sample() as i32;
        }
        if len == remaining && self.looping() {
            // chance to loop a song
            self.mix_into(&mut mixbuf[len..]);
        }
        */
    }

    fn remaining_samples(&self) -> usize {
        self.sample_count - self.decode_position
    }

    fn looping(&self) -> bool {
        self.looping
    }

    fn data_ptr(&self) -> *const u8 {
        self.data.as_ptr() as *const u8
    }

    fn reset(&mut self) {
        self.decode_position = 0;
    }
}
