// Adapted from `wgpu/examples/src/hello_triangle/shader.wgsl`
@group(0)
@binding(0)
var<uniform> canvas_width: f32;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec4<f32>, 3>(
        vec4<f32>(1.0 - 200.0 / canvas_width, 1.0, 0.0, 1.0),   // Top vertex
        vec4<f32>(-1.0, -1.0, 0.0, 1.0), // Bottom left vertex
        vec4<f32>(1.0, -1.0, 0.0, 1.0)   // Bottom right vertex
    );
    return positions[in_vertex_index];
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
