//! Linear RGBA colour, mirroring `THREE.Color` plus an alpha channel.

/// An RGBA colour with components in the `0.0..=1.0` range.
///
/// `Color` is a plain `Copy` value type: it holds no GPU state and can be
/// created, cloned and compared freely on the CPU.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Color = Color::rgba(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);
    pub const RED: Color = Color::rgba(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Color = Color::rgba(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Color = Color::rgba(0.0, 0.0, 1.0, 1.0);
    pub const YELLOW: Color = Color::rgba(1.0, 1.0, 0.0, 1.0);
    pub const CYAN: Color = Color::rgba(0.0, 1.0, 1.0, 1.0);
    pub const MAGENTA: Color = Color::rgba(1.0, 0.0, 1.0, 1.0);
    /// Fully transparent black — the default clear colour for a `Scene` is not
    /// this, but it is a useful neutral for blending.
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

    /// Creates an opaque colour from red/green/blue components.
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Color::rgba(r, g, b, 1.0)
    }

    /// Creates a colour from red/green/blue/alpha components.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color { r, g, b, a }
    }

    /// Creates an opaque colour from a packed `0xRRGGBB` integer.
    ///
    /// ```
    /// # use neptune::math::Color;
    /// assert_eq!(Color::hex(0xff0000), Color::RED);
    /// ```
    pub fn hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xff) as f32 / 255.0;
        let g = ((hex >> 8) & 0xff) as f32 / 255.0;
        let b = (hex & 0xff) as f32 / 255.0;
        Color::rgba(r, g, b, 1.0)
    }

    /// Creates a colour from a packed `0xRRGGBBAA` integer.
    pub fn hex_alpha(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xff) as f32 / 255.0;
        let g = ((hex >> 16) & 0xff) as f32 / 255.0;
        let b = ((hex >> 8) & 0xff) as f32 / 255.0;
        let a = (hex & 0xff) as f32 / 255.0;
        Color::rgba(r, g, b, a)
    }

    /// Returns a copy of this colour with the alpha channel replaced.
    pub const fn with_alpha(self, a: f32) -> Self {
        Color { a, ..self }
    }

    /// Linearly interpolates towards `other`. `t` is clamped to `0.0..=1.0`.
    pub fn lerp(self, other: Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// The component array the renderer hands to the GPU.
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Color::rgba(r, g, b, a)
    }
}

impl From<u32> for Color {
    fn from(hex: u32) -> Self {
        Color::hex(hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn new_is_opaque() {
        let c = Color::new(0.25, 0.5, 0.75);
        assert!(approx(c.r, 0.25));
        assert!(approx(c.g, 0.5));
        assert!(approx(c.b, 0.75));
        assert!(approx(c.a, 1.0));
    }

    #[test]
    fn hex_unpacks_channels() {
        let c = Color::hex(0x00ff88);
        assert!(approx(c.r, 0.0));
        assert!(approx(c.g, 1.0));
        assert!(approx(c.b, 0x88 as f32 / 255.0));
        assert!(approx(c.a, 1.0));
    }

    #[test]
    fn hex_matches_named_constants() {
        assert_eq!(Color::hex(0xff0000), Color::RED);
        assert_eq!(Color::hex(0x00ff00), Color::GREEN);
        assert_eq!(Color::hex(0x0000ff), Color::BLUE);
        assert_eq!(Color::hex(0xffffff), Color::WHITE);
        assert_eq!(Color::hex(0x000000), Color::BLACK);
    }

    #[test]
    fn hex_alpha_unpacks_four_channels() {
        let c = Color::hex_alpha(0x8040207f);
        assert!(approx(c.r, 0x80 as f32 / 255.0));
        assert!(approx(c.g, 0x40 as f32 / 255.0));
        assert!(approx(c.b, 0x20 as f32 / 255.0));
        assert!(approx(c.a, 0x7f as f32 / 255.0));
    }

    #[test]
    fn with_alpha_keeps_rgb() {
        let c = Color::RED.with_alpha(0.5);
        assert!(approx(c.r, 1.0));
        assert!(approx(c.a, 0.5));
    }

    #[test]
    fn lerp_endpoints_and_midpoint() {
        assert_eq!(Color::BLACK.lerp(Color::WHITE, 0.0), Color::BLACK);
        assert_eq!(Color::BLACK.lerp(Color::WHITE, 1.0), Color::WHITE);
        let mid = Color::BLACK.lerp(Color::WHITE, 0.5);
        assert!(approx(mid.r, 0.5) && approx(mid.g, 0.5) && approx(mid.b, 0.5));
    }

    #[test]
    fn lerp_clamps_out_of_range_t() {
        assert_eq!(Color::BLACK.lerp(Color::WHITE, -3.0), Color::BLACK);
        assert_eq!(Color::BLACK.lerp(Color::WHITE, 9.0), Color::WHITE);
    }

    #[test]
    fn to_array_round_trips() {
        let c = Color::rgba(0.1, 0.2, 0.3, 0.4);
        assert_eq!(Color::from(c.to_array()), c);
    }
}
