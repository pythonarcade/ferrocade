//! Per-sprite GPU instance data, laid out exactly as the vertex shader reads it.

use bytemuck::{Pod, Zeroable};

use crate::store::SpriteState;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Instance {
    /// Sprite center, in pixels.
    pub center: [f32; 2],
    /// Drawn size in pixels: base size (or texture size) × |scale|.
    pub size: [f32; 2],
    /// Degrees. Stored for later; not applied by the shader yet.
    pub angle: f32,
    /// Tint, RGBA 0-255 in sRGB (read as `Unorm8x4`; white leaves the texture unchanged).
    pub color: [u8; 4],
    pub _pad: [f32; 2],
    /// Texture region (u0, v0, u1, v1); the whole texture is (0, 0, 1, 1).
    pub uv_rect: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<Instance>() == 48);
const _: () = assert!(std::mem::offset_of!(Instance, color) == 20);
const _: () = assert!(std::mem::offset_of!(Instance, uv_rect) == 32);

impl Instance {
    pub fn from_sprite(sprite: &SpriteState) -> Self {
        let (width, height) = sprite.drawn_size();
        Self {
            center: [sprite.center_x as f32, sprite.center_y as f32],
            size: [width as f32, height as f32],
            angle: sprite.angle as f32,
            color: sprite.color,
            _pad: [0.0; 2],
            uv_rect: sprite.uv_rect,
        }
    }
}
