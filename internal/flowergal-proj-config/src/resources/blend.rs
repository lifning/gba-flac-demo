/// "a" is base layer, "b" is top layer
/// special thanks to https://en.wikipedia.org/wiki/Blend_modes#Overlay for existing,
/// as well as https://en.wikipedia.org/wiki/Alpha_compositing#Alpha_blending

pub mod fixed {
    use gba::Color;

    use fixed::types::extra::{U6, U8};
    use fixed::FixedU16;

    fn channel_to_fixed(c: u16) -> FixedU16<U8> {
        // FIXME: want [0.0, 1.0] inclusive, but this gives us [0.0, 1.0) non-inclusive
        FixedU16::from_bits(c << 3)
    }

    fn fixed_to_channel(f: FixedU16<U8>) -> u16 {
        (f.to_bits() >> 3).min(31)
    }

    fn color_to_fixed(c: Color) -> (FixedU16<U8>, FixedU16<U8>, FixedU16<U8>) {
        (
            channel_to_fixed(c.red()),
            channel_to_fixed(c.green()),
            channel_to_fixed(c.blue()),
        )
    }

    pub fn blend_multiply(a: Color, b: Color) -> Color {
        let (a_red, a_green, a_blue) = color_to_fixed(a);
        let (b_red, b_green, b_blue) = color_to_fixed(b);
        Color::from_rgb(
            fixed_to_channel(a_red * b_red),
            fixed_to_channel(a_green * b_green),
            fixed_to_channel(a_blue * b_blue),
        )
    }

    pub fn channel_overlay(a: FixedU16<U8>, b: FixedU16<U8>) -> u16 {
        if a < FixedU16::<U8>::from_bits(0b0_1000_0000) {
            fixed_to_channel(2 * a * b)
        } else {
            let one = FixedU16::<U8>::from_bits(0b1_0000_0000);
            fixed_to_channel(one - (2 * (one - a) * (one - b)))
        }
    }

    pub fn blend_overlay(a: Color, b: Color) -> Color {
        let (a_red, a_green, a_blue) = color_to_fixed(a);
        let (b_red, b_green, b_blue) = color_to_fixed(b);
        Color::from_rgb(
            channel_overlay(a_red, b_red),
            channel_overlay(a_green, b_green),
            channel_overlay(a_blue, b_blue),
        )
    }

    pub fn blend_hardlight(a: Color, b: Color) -> Color {
        blend_overlay(b, a)
    }

    pub fn blend_precomputed_alpha(
        a: Color,
        b_precomp: (FixedU16<U6>, FixedU16<U6>, FixedU16<U6>),
        inv_alpha: FixedU16<U6>,
    ) -> Color {
        let (b_red_precomp, b_green_precomp, b_blue_precomp) = b_precomp;
        let cr = b_red_precomp + (a.red() * inv_alpha);
        let cg = b_green_precomp + (a.green() * inv_alpha);
        let cb = b_blue_precomp + (a.blue() * inv_alpha);
        Color::from_rgb(cr.to_num(), cg.to_num(), cb.to_num())
    }
}

#[cfg(not(target_arch = "arm"))]
pub mod float {
    type Rgb888 = (u8, u8, u8);

    fn gba_channel_to_float(c: u16) -> f64 {
        c as f64 / 31.0
    }

    fn rgb888_channel_to_float(c: u8) -> f64 {
        c as f64 / 255.0
    }

    pub fn float_to_gba_channel(f: f64) -> u16 {
        let c = (f * 31.0).round() as u16;
        c.min(31)
    }

    pub fn gba_color_to_float(c: gba::Color) -> (f64, f64, f64) {
        (
            gba_channel_to_float(c.red()),
            gba_channel_to_float(c.green()),
            gba_channel_to_float(c.blue()),
        )
    }

    fn rgb888_color_to_float((r, g, b): Rgb888) -> (f64, f64, f64) {
        (
            rgb888_channel_to_float(r),
            rgb888_channel_to_float(g),
            rgb888_channel_to_float(b),
        )
    }

    pub fn blend_multiply(a: gba::Color, b: Rgb888) -> gba::Color {
        let (a_red, a_green, a_blue) = gba_color_to_float(a);
        let (b_red, b_green, b_blue) = rgb888_color_to_float(b);
        gba::Color::from_rgb(
            float_to_gba_channel(a_red * b_red),
            float_to_gba_channel(a_green * b_green),
            float_to_gba_channel(a_blue * b_blue),
        )
    }

    pub fn channel_overlay(a: f64, b: f64) -> u16 {
        if a < 0.5 {
            float_to_gba_channel(2.0 * a * b)
        } else {
            float_to_gba_channel(1.0 - (2.0 * (1.0 - a) * (1.0 - b)))
        }
    }

    pub fn channel_alpha(a: f64, b: f64, alpha: f64) -> u16 {
        float_to_gba_channel((b * alpha) + (a * (1.0 - alpha)))
    }

    pub fn blend_overlay(a: gba::Color, b: Rgb888) -> gba::Color {
        let (a_red, a_green, a_blue) = gba_color_to_float(a);
        let (b_red, b_green, b_blue) = rgb888_color_to_float(b);
        gba::Color::from_rgb(
            channel_overlay(a_red, b_red),
            channel_overlay(a_green, b_green),
            channel_overlay(a_blue, b_blue),
        )
    }

    pub fn blend_hardlight(a: gba::Color, b: Rgb888) -> gba::Color {
        let (a_red, a_green, a_blue) = gba_color_to_float(a);
        let (b_red, b_green, b_blue) = rgb888_color_to_float(b);
        gba::Color::from_rgb(
            // swapped b and a
            channel_overlay(b_red, a_red),
            channel_overlay(b_green, a_green),
            channel_overlay(b_blue, a_blue),
        )
    }

    pub fn blend_alpha(a: gba::Color, b: Rgb888, alpha: f64) -> gba::Color {
        let (a_red, a_green, a_blue) = gba_color_to_float(a);
        let (b_red, b_green, b_blue) = rgb888_color_to_float(b);
        gba::Color::from_rgb(
            // swapped b and a
            channel_alpha(a_red, b_red, alpha),
            channel_alpha(a_green, b_green, alpha),
            channel_alpha(a_blue, b_blue, alpha),
        )
    }
}
