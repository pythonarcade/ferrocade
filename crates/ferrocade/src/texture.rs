//! Textures: decoded images shared between sprites, and the texture filter choice.

/// How a texture is sampled when drawn.
///
/// There is deliberately no `Default`: the right choice depends on the game's art style, so the
/// window requires an explicit filter and textures may override it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Filter {
    /// Crisp pixels with no smoothing (pixel art).
    Nearest,
    /// Smooth interpolation (photos, scaled or rotated art).
    Linear,
}
