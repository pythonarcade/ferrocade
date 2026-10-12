//! Color parsing. Colors are RGB or RGBA tuples of 0-255 integers.

use crate::{Error, Result};

/// Background used when a frame has no `clear()` call.
pub const DARK_GRAY: [u8; 4] = [32, 32, 32, 255];

/// Validate a 3- or 4-component color (alpha defaults to 255).
pub fn parse_rgba(components: &[i64]) -> Result<[u8; 4]> {
    if components.len() != 3 && components.len() != 4 {
        return Err(Error::InvalidArgument(format!(
            "color must have 3 or 4 components (RGB or RGBA), got {}",
            components.len()
        )));
    }
    let mut rgba = [0u8, 0, 0, 255];
    for (i, &c) in components.iter().enumerate() {
        rgba[i] = u8::try_from(c).map_err(|_| {
            Error::InvalidArgument(format!("color components must be 0-255, got {c}"))
        })?;
    }
    Ok(rgba)
}

/// Convert to the linear-space floats wgpu expects for clear colors on an sRGB surface.
pub fn to_linear_f32(rgba: [u8; 4]) -> [f64; 4] {
    let srgb_to_linear = |c: u8| {
        let c = c as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [
        srgb_to_linear(rgba[0]),
        srgb_to_linear(rgba[1]),
        srgb_to_linear(rgba[2]),
        rgba[3] as f64 / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rgb_and_rgba() {
        assert_eq!(parse_rgba(&[1, 2, 3]).unwrap(), [1, 2, 3, 255]);
        assert_eq!(parse_rgba(&[1, 2, 3, 4]).unwrap(), [1, 2, 3, 4]);
    }

    #[test]
    fn rejects_bad_colors() {
        assert!(parse_rgba(&[1, 2]).is_err());
        assert!(parse_rgba(&[1, 2, 3, 4, 5]).is_err());
        assert!(parse_rgba(&[300, 0, 0]).is_err());
        assert!(parse_rgba(&[-1, 0, 0]).is_err());
    }

    #[test]
    fn linear_conversion_endpoints() {
        assert_eq!(to_linear_f32([0, 0, 0, 0]), [0.0, 0.0, 0.0, 0.0]);
        let white = to_linear_f32([255, 255, 255, 255]);
        assert!(white.iter().all(|&c| (c - 1.0).abs() < 1e-9));
        // Mid-gray in sRGB is much darker in linear space.
        assert!((to_linear_f32([128, 0, 0, 255])[0] - 0.2158).abs() < 1e-3);
    }
}
