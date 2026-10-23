// Presentation: scales GM's back buffer into the window (by a whole number when it fits), black around it.

struct Present {
    // Top-left of the scaled image in window pixels, and the scale.
    offset: vec2<f32>,
    scale: f32,
    _pad: f32,
}

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var<uniform> present: Present;

@vertex
fn vs_present(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    // One triangle that covers the window.
    let xy = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(xy * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_present(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = floor((pos.xy - present.offset) / present.scale);
    let size = vec2<f32>(textureDimensions(screen));
    if any(p < vec2<f32>(0.0)) || any(p >= size) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let c = textureLoad(screen, vec2<i32>(p), 0);
    // The window is opaque whatever alpha GM left in its back buffer.
    return vec4<f32>(c.rgb, 1.0);
}
