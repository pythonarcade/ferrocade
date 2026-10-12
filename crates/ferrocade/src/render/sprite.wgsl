// Instanced textured quads: one instance per sprite.

struct Globals {
    // Maps logical pixels (origin bottom-left, y up) to clip space.
    projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct VertexIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) center: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) angle: f32,     // degrees; not applied yet
    @location(3) uv_rect: vec4<f32>,
    @location(4) color: vec4<f32>,  // tint, sRGB in 0..1 (from Unorm8x4)
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,  // tint, linear
};

// Exact sRGB -> linear conversion, so a white texture tinted (r, g, b) writes back exactly
// (r, g, b) through the sRGB render target.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    // Two triangles covering the unit square; (0, 0) is the sprite's bottom-left corner.
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let corner = corners[in.vertex_index];
    let world = in.center + (corner - vec2<f32>(0.5, 0.5)) * in.size;

    var out: VertexOut;
    out.position = globals.projection * vec4<f32>(world, 0.0, 1.0);
    // Texture rows start at the top, so the sprite's top edge (corner.y = 1) uses v0.
    out.uv = vec2<f32>(
        mix(in.uv_rect.x, in.uv_rect.z, corner.x),
        mix(in.uv_rect.w, in.uv_rect.y, corner.y),
    );
    out.color = vec4<f32>(srgb_to_linear(in.color.rgb), in.color.a);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, in.uv) * in.color;
}
