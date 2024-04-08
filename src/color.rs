use std::ops::{Add, Mul};

use glam::{Vec3, Vec4};

use super::lerp::Lerp;

/// An SRGB color.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Color {
    /// Red
    pub r: f32,
    /// Green
    pub g: f32,
    /// Blue
    pub b: f32,
    /// Alpha
    pub a: f32,
}

impl Lerp for Color {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Self {
            r: self.r.lerp(&other.r, factor),
            g: self.g.lerp(&other.g, factor),
            b: self.b.lerp(&other.b, factor),
            a: self.a.lerp(&other.a, factor),
        }
    }
}

impl Color {
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0);
    pub const LIGHTGREY: Color = Color::new(0.7, 0.7, 0.75);
    pub const DARKGREY: Color = Color::new(0.1, 0.1, 0.15);
    pub const GREY: Color = Color::new(0.4, 0.4, 0.5);
    pub const RED: Color = Color::new(1.0, 0.0, 0.0);
    pub const ORANGE: Color = Color::new(1.0, 0.6, 0.0);
    pub const GREEN: Color = Color::new(0.0, 1.0, 0.0);
    pub const DARKGREEN: Color = Color::new(0.1, 0.3, 0.1);
    pub const BLUE: Color = Color::new(0.0, 0.0, 1.0);
    pub const LIGHTBLUE: Color = Color::new(0.4, 0.4, 1.0);
    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0);
    pub const YELLOW: Color = Color::new(1.0, 1.0, 0.0);
    pub const PURPLE: Color = Color::new(1.0, 0.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Color { r, g, b, a: 1.0 }
    }

    pub fn mix(self, other: Color, factor: f32) -> Self {
        self.lerp(&other, factor)
    }

    /// Converts values in HSV color space to RGB
    ///
    /// * hue: f64 - the position of the color on the color wheel. Between 0 and 360
    /// * saturation: f64 - how much color. Between 0, no color, and 1, all color
    /// * value: f64 - or lightness. Between 0, black, and 1, white
    pub fn from_hsv(hue: f64, saturation: f64, value: f64) -> Self {
        hsv_to_rgb(hue, saturation, value)
    }

    #[inline]
    pub fn from_hex(hex: &str) -> Color {
        const fn hex_digit_value(c: char) -> u8 {
            match c {
                '0'..='9' => c as u8 - b'0',
                'a'..='f' => c as u8 - b'a' + 10,
                'A'..='F' => c as u8 - b'A' + 10,
                _ => 0,
            }
        }

        const fn parse_hex_pair(s: &str, start: usize) -> u8 {
            16 * hex_digit_value(s.as_bytes()[start] as char)
                + hex_digit_value(s.as_bytes()[start + 1] as char)
        }

        if hex.as_bytes()[0] != "#".as_bytes()[0] {
            panic!("Hex string needs to start with #")
        }

        if hex.len() == 7 {
            let r = color_map_to_srgb(parse_hex_pair(hex, 1));
            let g = color_map_to_srgb(parse_hex_pair(hex, 3));
            let b = color_map_to_srgb(parse_hex_pair(hex, 5));
            Color { r, g, b, a: 1.0 }
        } else {
            panic!("Cannot create Color! Expects a hex string")
        }
    }

    /// creates colors from rgb and maps them into srgb space
    ///
    /// srgb_color = ((rgb_color / 255 + 0.055) / 1.055) ^ 2.4
    pub fn u8_srgb(r: u8, g: u8, b: u8) -> Self {
        Color {
            r: color_map_to_srgb(r),
            g: color_map_to_srgb(g),
            b: color_map_to_srgb(b),
            a: 1.0,
        }
    }

    pub const fn alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

/// srgb_color = ((rgb_color / 255 + 0.055) / 1.055) ^ 2.4
#[inline]
pub fn color_map_to_srgb(u: u8) -> f32 {
    // u as f32 / 255.0
    ((u as f32 / 255.0 + 0.055) / 1.055).powf(2.4)
}

impl From<Color> for wgpu::Color {
    fn from(value: Color) -> Self {
        wgpu::Color {
            r: value.r as f64,
            g: value.g as f64,
            b: value.b as f64,
            a: value.a as f64,
        }
    }
}

impl From<Vec3> for Color {
    fn from(value: Vec3) -> Self {
        Color {
            r: value.x,
            g: value.y,
            b: value.z,
            a: 1.0,
        }
    }
}

impl From<[f32; 3]> for Color {
    fn from(value: [f32; 3]) -> Self {
        Color {
            r: value[0],
            g: value[1],
            b: value[2],
            a: 1.0,
        }
    }
}

impl From<Vec4> for Color {
    fn from(value: Vec4) -> Self {
        Color {
            r: value.x,
            g: value.y,
            b: value.z,
            a: value.w,
        }
    }
}

impl Mul<Color> for Color {
    type Output = Color;

    fn mul(self, rhs: Color) -> Self::Output {
        Self {
            r: self.r * rhs.r,
            g: self.g * rhs.g,
            b: self.b * rhs.b,
            a: self.a * rhs.a,
        }
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a,
        }
    }
}

impl Add<f32> for Color {
    type Output = Color;

    fn add(self, rhs: f32) -> Self::Output {
        Self {
            r: self.r + rhs,
            g: self.g + rhs,
            b: self.b + rhs,
            a: self.a,
        }
    }
}

/// Credit: https://github.com/jayber/hsv/blob/main/src/lib.rs
///
/// Converts values in HSV color space to RGB
///
/// * hue: f64 - the position of the color on the color wheel. Between 0 and 360
/// * saturation: f64 - how much color. Between 0, no color, and 1, all color
/// * value: f64 - or lightness. Between 0, black, and 1, white
///
/// # Panics
/// If the supplied values are outside the ranges stated above. The ranges are inclusive
///
/// ## Examples
/// - Black = 0.0, 0.0, 0.0
/// - White = 0.0, 0.0, 1.0
/// - Red = 0.0, 1.0, 1.0
/// - Green = 120.0, 1.0, 1.0
/// - Blue = 240.0, 1.0, 1.0
pub fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> Color {
    fn is_between(value: f64, min: f64, max: f64) -> bool {
        min <= value && value < max
    }

    // check_bounds(hue, saturation, value);

    let c = value * saturation;
    let h = hue / 60.0;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = value - c;

    let (r, g, b): (f64, f64, f64) = if is_between(h, 0.0, 1.0) {
        (c, x, 0.0)
    } else if is_between(h, 1.0, 2.0) {
        (x, c, 0.0)
    } else if is_between(h, 2.0, 3.0) {
        (0.0, c, x)
    } else if is_between(h, 3.0, 4.0) {
        (0.0, x, c)
    } else if is_between(h, 4.0, 5.0) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    return Color::new((r + m) as f32, (g + m) as f32, (b + m) as f32);

    fn check_bounds(hue: f64, saturation: f64, value: f64) {
        fn panic_bad_params(name: &str, from_value: &str, to_value: &str, supplied: f64) -> ! {
            panic!(
                "param {} must be between {} and {} inclusive; was: {}",
                name, from_value, to_value, supplied
            )
        }

        if !(0.0..=360.0).contains(&hue) {
            panic_bad_params("hue", "0.0", "360.0", hue)
        } else if !(0.0..=1.0).contains(&saturation) {
            panic_bad_params("saturation", "0.0", "1.0", saturation)
        } else if !(0.0..=1.0).contains(&value) {
            panic_bad_params("value", "0.0", "1.0", value)
        }
    }
}
