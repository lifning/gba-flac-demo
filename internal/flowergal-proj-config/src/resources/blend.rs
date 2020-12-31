/// "a" is base layer, "b" is top layer
/// special thanks to https://en.wikipedia.org/wiki/Blend_modes#Overlay for existing,
/// as well as https://en.wikipedia.org/wiki/Alpha_compositing#Alpha_blending

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
