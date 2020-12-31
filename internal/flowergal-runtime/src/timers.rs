use gba::io::timers::{TM3CNT_H, TM2CNT_H, TimerControlSetting, TimerTickRate, TM3CNT_L, TM2CNT_L, TM1CNT_H, TM1CNT_L};

pub struct GbaTimer {}

impl GbaTimer {
    pub const fn new() -> Self {
        GbaTimer{}
    }

    pub fn initialize(&self) {
        TM2CNT_H.write(TimerControlSetting::new());
        TM3CNT_H.write(TimerControlSetting::new());

        TM2CNT_L.write(0);
        TM3CNT_L.write(0);

        TM2CNT_H.write(TimerControlSetting::new()
            .with_enabled(true)
            .with_tick_rate(TimerTickRate::CPU64));
        TM3CNT_H.write(TimerControlSetting::new()
            .with_enabled(true)
            .with_tick_rate(TimerTickRate::Cascade));
    }

    pub fn setup_timer1_irq(cycles: u16) {
        TM1CNT_L.write(0u16.overflowing_sub(cycles).0);
        TM1CNT_H.write(TimerControlSetting::new()
            .with_tick_rate(TimerTickRate::CPU1)
            .with_overflow_irq(true)
            .with_enabled(true));
    }

    pub fn timer1() {
        TM1CNT_H.write(TimerControlSetting::new());
    }

    #[inline(always)]
    pub fn get_ticks() -> u32 {
        unsafe { asm!("", options(nostack)); } // prevent compiler memory reordering
        ((TM3CNT_L.read() as u32) << 16) | TM2CNT_L.read() as u32
    }
}
