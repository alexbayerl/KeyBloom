//! Provides utility functions for converting and manipulating colors between different color spaces.
//!
//! This module wraps conversions between the OpenRGB `Color` type and the `palette` crate’s
//! `Srgb` and `Hsv` color spaces. It also includes functions for color interpolation and
//! adjustments (brightness and saturation).

use openrgb::data::Color;
use palette::{FromColor, Hsv, RgbHue, Srgb};
use palette::IntoColor;

/// Convert an OpenRGB `Color` to a palette `Srgb<f32>`.
pub fn color_to_srgb(color: Color) -> Srgb<f32> {
    Srgb::new(
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
    )
}

/// Convert a palette `Srgb<f32>` to an OpenRGB `Color`.
pub fn srgb_to_color(srgb: Srgb<f32>) -> Color {
    let r = (srgb.red * 255.0).clamp(0.0, 255.0).round() as u8;
    let g = (srgb.green * 255.0).clamp(0.0, 255.0).round() as u8;
    let b = (srgb.blue * 255.0).clamp(0.0, 255.0).round() as u8;
    Color { r, g, b }
}

/// Interpolate between two colors in HSV space, with t in [0.0..1.0].
///
/// # Arguments
///
/// * `start` - The starting color (`Srgb<f32>`).
/// * `end` - The ending color (`Srgb<f32>`).
/// * `t` - Interpolation amount (0.0 = start, 1.0 = end).
///
/// # Returns
///
/// An `Srgb<f32>` that represents the color at the given interpolation amount.
pub fn interpolate_color_hsv(start: Srgb<f32>, end: Srgb<f32>, t: f32) -> Srgb<f32> {
    let shsv = Hsv::from_color(start);
    let ehsv = Hsv::from_color(end);

    // Handle potential hue wrap-around
    let shue_deg = shsv.hue.into_degrees();
    let ehue_deg = ehsv.hue.into_degrees();
    let mut delta_hue = ehue_deg - shue_deg;
    if delta_hue > 180.0 {
        delta_hue -= 360.0;
    } else if delta_hue < -180.0 {
        delta_hue += 360.0;
    }

    let interp_hue = shue_deg + delta_hue * t;
    let interp_saturation = shsv.saturation + (ehsv.saturation - shsv.saturation) * t;
    let interp_value = shsv.value + (ehsv.value - shsv.value) * t;

    Hsv::new(
        RgbHue::from_degrees(interp_hue.rem_euclid(360.0)),
        interp_saturation.clamp(0.0, 1.0),
        interp_value.clamp(0.0, 1.0),
    )
    .into_color()
}

/// Increase the saturation of an `Srgb<f32>` color by a given factor, clamping at 1.0.
pub fn adjust_saturation(srgb: Srgb<f32>, factor: f32) -> Srgb<f32> {
    let mut hsv = Hsv::from_color(srgb);
    hsv.saturation = (hsv.saturation * factor).clamp(0.0, 1.0);
    hsv.into_color()
}

/// Increase the brightness of an `Srgb<f32>` color by a given factor, clamping at 1.0.
pub fn increase_brightness(srgb: Srgb<f32>, factor: f32) -> Srgb<f32> {
    let mut hsv = Hsv::from_color(srgb);
    hsv.value = (hsv.value * factor).clamp(0.0, 1.0);
    hsv.into_color()
}
