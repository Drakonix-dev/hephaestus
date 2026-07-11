struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) uv: vec2<f32>,
};

struct VertexOutput {
  @builtin(position) clip_pos: vec4<f32>,
  @location(0) normal: vec3<f32>,
};

struct Globals {
  view_proj: mat4x4<f32>,
};

struct MaterialUniforms {
  base_color: vec4<f32>,
};

struct Model {
  model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(1) @binding(0)
var<uniform> material: MaterialUniforms;

@group(2) @binding(0)
var<uniform> model: Model;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  out.clip_pos = globals.view_proj * model.model * vec4(input.position, 1.0);
  out.normal = normalize((model.model * vec4(input.normal, 0.0)).xyz);
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let light_dir = normalize(vec3(0.5, 1.0, 0.5));
  let diff = max(dot(in.normal, light_dir), 0.0);
  let lit = diff * 0.8 + 0.2;
  return material.base_color * lit;
}
