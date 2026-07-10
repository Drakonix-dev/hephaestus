struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) uv: vec2<f32>,
};

struct VertexOutput {
  @builtin(position) clip_pos: vec4<f32>,
  @location(0) normal: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> material: MaterialUniforms;

struct MaterialUniforms {
  base_color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  out.clip_pos = vec4(input.position, 1.0);
  out.normal = input.normal;
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let light_dir = normalize(vec3(0.5, 1.0, 0.5));
  let diff = max(dot(in.normal, light_dir), 0.0);
  return material.base_color * diff;
}
