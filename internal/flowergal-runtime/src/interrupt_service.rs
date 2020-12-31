use crate::Driver;
use gba::io::display::{DisplayStatusSetting, DISPSTAT};
use gba::io::irq::{IrqEnableSetting, IrqFlags, BIOS_IF, IE, IF, IME, USER_IRQ_FN};
use crate::timers::GbaTimer;

pub fn irq_setup() {
    DISPSTAT.write(
        DisplayStatusSetting::new()
            .with_vblank_irq_enable(true)
            //.with_hblank_irq_enable(true)
            .with_vcounter_irq_enable(true)
            .with_vcount_setting(227),
    );

    IE.write(
        IrqFlags::new()
            .with_vblank(true)
            .with_vcounter(true)
            .with_timer1(true),
    );

    USER_IRQ_FN.write(Some(irq_handler));
    IME.write(IrqEnableSetting::IRQ_YES);

    warn!("Enabled interrupts"); // FIXME: load-bearing log!
}

#[link_section = ".iwram"]
#[instruction_set(arm::a32)]
fn irq_handler() {
    let flags = IF.read();
    //let mut handled = IF.read();

    let driver = unsafe { Driver::instance_mut() };

    if flags.vblank() {
        driver.audio().dsound_vblank();
        // driver.audio().mixer(); // testing performance..  seems way faster when run here??
        driver.video().vblank();
        //handled = handled.with_vblank(true);
    }
    /*
    if flags.hblank() {
        driver.video().hblank(VCOUNT.read());
        handled = handled.with_hblank(true);
    }
    */
    if flags.vcounter() {
        driver.video().vcounter();
        //handled = handled.with_vcounter(true);
    }
    if flags.timer1() {
        driver.video().timer1();
        GbaTimer::timer1();
    }
    IF.write(flags);
    BIOS_IF.write(flags);
}
