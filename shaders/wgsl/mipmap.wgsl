struct VertexInput {
    @location(0) position : vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(
    model : VertexInput
) -> VertexOutput {
    var out : VertexOutput;
    
    out.uv = vec2<f32>(model.position.x / 2.0 + 0.5, -model.position.y / 2.0 + 0.5);
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

struct FragmentOutput {
@location(0) diffuse : vec4<f32>,
};

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

struct ScreenSize {
    size : vec2<f32>
};

@group(0) @binding(2)
var<uniform> screen : ScreenSize;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;

    let step = vec2<f32>(1.0,1.0) / screen.size;

    var res = textureSample(t_diffuse, s_diffuse, in.uv);
    res += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(1.0, 0.0) * step);
    res += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(1.0, 1.0) * step);
    res += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, 1.0) * step);
    res = res / 4.0;

    out.diffuse = res;

    return out;
}