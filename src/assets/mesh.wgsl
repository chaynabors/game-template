struct PushConstants {
    mvp: mat4x4<f32>,
}

struct VertexOutput {
    @location(0) color: vec3<f32>,
    @builtin(position) position: vec4<f32>,
};

var<push_constant> push_constants: PushConstants;

@vertex
fn vs_main(
    @location(0) position: vec4<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.color = color;
    result.position = push_constants.mvp * position;
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(vertex.color, 1.0);
}
