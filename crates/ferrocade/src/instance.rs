//! Per-sprite GPU instance data, laid out exactly as the vertex shader reads it.

use bytemuck::{Pod, Zeroable};

use crate::store::SpriteState;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Instance {
    /// Sprite center, in pixels.
    pub center: [f32; 2],
    /// Drawn size in pixels: texture size × |scale|.
    pub size: [f32; 2],
    /// Degrees. Stored for later; not applied by the Phase 1 shader.
    pub angle: f32,
    pub _pad0: [f32; 3],
    /// Texture region (u0, v0, u1, v1); the whole texture is (0, 0, 1, 1).
    pub uv_rect: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<Instance>() == 48);

impl Instance {
    pub fn from_sprite(sprite: &SpriteState) -> Self {
        let scale = sprite.scale.abs();
        let texture = &sprite.texture.data;
        Self {
            center: [sprite.center_x as f32, sprite.center_y as f32],
            size: [
                (texture.width as f64 * scale) as f32,
                (texture.height as f64 * scale) as f32,
            ],
            angle: sprite.angle as f32,
            _pad0: [0.0; 3],
            uv_rect: [0.0, 0.0, 1.0, 1.0],
        }
    }
}
