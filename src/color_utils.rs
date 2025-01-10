//! Provides utility functions for converting and manipulating colors between different color spaces.
//!
//! This module wraps conversions between the OpenRGB `Color` type and the `palette` crate’s
//! `Srgb` and `Hsv` color spaces. It also includes functions for color interpolation and
//! adjustments (brightness and saturation).

use openrgb::data::Color;
use palette::{FromColor, Hsv, RgbHue, Srgb};

/// Convert an OpenRGB `Color` to a palette `Srgb<f32>`.
///
/// # Arguments
///
/// * `color` - An OpenRGB `Color` struct containing RGB values in `u8` (0-255).
///
/// # Returns
///
/// An `Srgb<f32>` with all components normalized to 0.0-1.0.
pub fn color_to_srgb(color: Color) -> Srgb<f32> {
    Srgb::new(
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
    )
}

/// Convert a palette `Srgb<f32>` to an OpenRGB `Color`.
///
/// # Arguments
///
/// * `srgb` - A color in `Srgb<f32>` format.
///
/// # Returns
///
/// An OpenRGB `Color` struct with RGB values clamped and converted to `u8` (0-255).
pub fn srgb_to_color(srgb: Srgb<f32>) -> Color {
    let r = (srgb.red * 255.0).clamp(0.0, 255.0).round() as u8;
    let g = (srgb.green * 255.0).clamp(0.0, 255.0).round() as u8;
    let b = (srgb.blue * 255.0).clamp(0.0, 255.0).round() as u8;
    Color { r, g, b }
}

/// Convert an `Srgb<f32>` to `Hsv`.
///
/// # Arguments
///
/// * `s` - A color in `Srgb<f32>` format.
///
/// # Returns
///
/// A color in the `Hsv` color space for easier manipulation of hue, saturation, and value.
pub fn srgb_to_hsv(s: Srgb<f32>) -> Hsv {
    Hsv::from_color(s)
}

/// Convert an `Hsv` color to `Srgb<f32>`.
///
/// # Arguments
///
/// * `hsv` - A color in `Hsv` format.
///
/// # Returns
///
/// An `Srgb<f32>` representation of the same color.
pub fn hsv_to_srgb(hsv: Hsv) -> Srgb<f32> {
    Srgb::from_color(hsv)
}

/// Interpolate between two colors in HSV space.
///
/// This function calculates an intermediate color based on the given step.
/// If hue values wrap around (e.g., from 350° to 10°), the function
/// automatically adjusts for the shortest rotation direction.
///
/// # Arguments
///
/// * `start` - The starting color (`Srgb<f32>`).
/// * `end` - The ending color (`Srgb<f32>`).
/// * `step` - The current step in the interpolation (0-based).
/// * `total_steps` - The total number of steps for the entire interpolation.
///
/// # Returns
///
/// An `Srgb<f32>` that represents the color at the given interpolation step.
pub fn interpolate_color_hsv(
    start: Srgb<f32>,
    end: Srgb<f32>,
    step: usize,
    total_steps: usize,
) -> Srgb<f32> {
    if total_steps == 0 {
        return start;
    }

    let shsv = srgb_to_hsv(start);
    let ehsv = srgb_to_hsv(end);

    // Handle potential hue wrap-around
    let shue_deg = shsv.hue.into_degrees();
    let ehue_deg = ehsv.hue.into_degrees();
    let mut delta_hue = ehue_deg - shue_deg;

    if delta_hue > 180.0 {
        delta_hue -= 360.0;
    } else if delta_hue < -180.0 {
        delta_hue += 360.0;
    }

    let t = step as f32 / total_steps as f32;
    let interp_hue = shue_deg + delta_hue * t;
    let interp_saturation = shsv.saturation + (ehsv.saturation - shsv.saturation) * t;
    let interp_value = shsv.value + (ehsv.value - shsv.value) * t;

    hsv_to_srgb(Hsv::new(
        RgbHue::from_degrees(interp_hue.rem_euclid(360.0)),
        interp_saturation.clamp(0.0, 1.0),
        interp_value.clamp(0.0, 1.0),
    ))
}

/// Increase the saturation of an `Srgb<f32>` color by a given factor, clamping at 1.0.
///
/// # Arguments
///
/// * `srgb` - A color in `Srgb<f32>` format.
/// * `factor` - The factor by which to multiply the saturation.
///
/// # Returns
///
/// A new color in `Srgb<f32>` with modified saturation.
pub fn adjust_saturation(srgb: Srgb<f32>, factor: f32) -> Srgb<f32> {
    let hsv = srgb_to_hsv(srgb);
    let new_sat = (hsv.saturation * factor).clamp(0.0, 1.0);
    hsv_to_srgb(Hsv::new(hsv.hue, new_sat, hsv.value))
}

/// Increase the brightness of an `Srgb<f32>` color by a given factor, clamping at 1.0.
///
/// # Arguments
///
/// * `srgb` - A color in `Srgb<f32>` format.
/// * `factor` - The factor by which to multiply the brightness (value).
///
/// # Returns
///
/// A new color in `Srgb<f32>` with an adjusted brightness.
pub fn increase_brightness(srgb: Srgb<f32>, factor: f32) -> Srgb<f32> {
    let hsv = srgb_to_hsv(srgb);
    let new_val = (hsv.value * factor).clamp(0.0, 1.0);
    hsv_to_srgb(Hsv::new(hsv.hue, hsv.saturation, new_val))
}
