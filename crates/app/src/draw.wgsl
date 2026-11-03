// Draws GM primitives. Positions arrive in clip space (the CPU applies the view), texture coordinates in
// texels. Sampling is a texel fetch, never filtered: transparent atlas pixels keep whatever RGB they were
// stored with, so any filtering would bleed it in.

struct VertexIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    // The image's texel rectangle (x0, y0, x1, y1), exclusive at the far edge.
    @location(3) rect: vec4<u32>,
    // Array layer, with BACKGROUND_BANK set for the background array; or UNTEXTURED for plain colour.
    @location(4) layer: u32,
}

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) rect: vec4<u32>,
    @location(3) @interpolate(flat) layer: u32,
}

const UNTEXTURED: u32 = 0xFFFFFFFFu;
const BACKGROUND_BANK: u32 = 0x80000000u;

// Sprite atlas pages, or the surface being drawn.
@group(0) @binding(0) var sprites: texture_2d_array<f32>;
@group(0) @binding(1) var backgrounds: texture_2d_array<f32>;

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.pos = vec4<f32>(v.pos, 0.0, 1.0);
    out.uv = v.uv;
    out.color = v.color;
    out.rect = v.rect;
    out.layer = v.layer;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    if v.layer == UNTEXTURED {
        return v.color;
    }
    // A sample exactly on a texel boundary takes the later texel, as Direct3D 8's point sampling does: under a 2x
    // view every other pixel's sample is on one, and the interpolated f32 may land a hair short of it, so it's
    // nudged by 1/512 of a texel (851-3967). Then clamped to the image like GM's renderer does, so an edge pixel
    // never reads its neighbour.
    let uv = floor(v.uv + vec2<f32>(1.0 / 512.0));
    let texel = clamp(vec2<i32>(uv), vec2<i32>(v.rect.xy), vec2<i32>(v.rect.zw) - 1);
    let layer = i32(v.layer & ~BACKGROUND_BANK);
    if (v.layer & BACKGROUND_BANK) != 0u {
        return textureLoad(backgrounds, texel, layer, 0) * v.color;
    }
    return textureLoad(sprites, texel, layer, 0) * v.color;
}
