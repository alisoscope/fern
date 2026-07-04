struct Globals {
	view_proj: mat4x4<f32>,
	x: f32,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(0) @binding(1)
var<storage, read> model: array<mat4x4<f32>>;

struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) normal: vec3<f32>,
	@location(2) tex_coord: vec2<f32>,
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) colour: f32,
	@location(1) normal: vec3<f32>,
};

const positions = array<vec2<f32>, 3>(
	vec2(0.0, -0.5),
	vec2(0.5, 0.5),
	vec2(-0.5, 0.5),
);

const view_proj_const = mat4x4<f32>(
	-0.71606636, 0, 0, 0,
	0, -5.33511e-8, -1, -1,
	0, 0.89508295, -5.960465e-8, -5.960465e-8,
	0, -5.3351107e-7, -10.1, -10.0,
);

@vertex
fn vertex_main(
	@builtin(instance_index) instance_index: u32,
	vertex: VertexInput,
) -> VertexOutput {
	var out: VertexOutput;

	//out.clip_position = view_proj * vec4(vertex.position, 1.0);
	out.clip_position = globals.view_proj * model[instance_index] * vec4(vertex.position, 1.0);
	//out.clip_position = view_proj_const * vec4(vertex.position, 1.0);
	out.colour = globals.x;
	out.normal = vertex.normal;

	return out;
}

@fragment
fn fragment_main(
	in: VertexOutput
) -> @location(0) vec4<f32> {
	return vec4<f32>(in.normal, 1.0);
	return vec4<f32>(in.colour, 0.1, 0.1, 1.0);
}
