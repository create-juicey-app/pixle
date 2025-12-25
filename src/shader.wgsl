struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // CHANGE 1: Use 'var', not 'let' so we can index it dynamically
    var pos = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0), // Bottom-Left
        vec2<f32>( 1.0, -1.0), // Bottom-Right
        vec2<f32>(-1.0,  1.0), // Top-Left
        vec2<f32>( 1.0,  1.0)  // Top-Right
    );

    // CHANGE 2: Use 'var', not 'let'
    var indices = array<u32, 6>(0u, 1u, 2u, 2u, 1u, 3u);

    // Now this is legal because 'indices' is a variable in memory
    let index = indices[in_vertex_index];

    let xy = pos[index];
    out.clip_position = vec4<f32>(xy, 0.0, 1.0);

    out.tex_coords = vec2<f32>(xy.x * 0.5 + 0.5, 1.0 - (xy.y * 0.5 + 0.5));

    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
