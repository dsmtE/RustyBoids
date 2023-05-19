struct InitParameters {
  seed: u32,
}

struct SimulationParameters {
  boids_count: u32,
  delta_t: f32,
  view_radius: f32,
  cohesion_scale: f32,
  aligment_scale: f32,
  separation_scale: f32,
  grid_size: u32,
}

@group(0) @binding(0) var<uniform> initParameters : InitParameters;
@group(1) @binding(0) var<uniform> simulationParameters : SimulationParameters;

// @group(1) @binding(0) var<storage, read> boidsPositionSrc : array<vec2<f32>>;
// @group(1) @binding(1) var<storage, read> boidsVelocitySrc : array<vec2<f32>>;
// @group(1) @binding(2) var<storage, read> boidsCellIdSrc : array<vec2<u32>>;

@group(2) @binding(3) var<storage, read_write> boidsPositionDst : array<vec2<f32>>;
@group(2) @binding(4) var<storage, read_write> boidsVelocityDst : array<vec2<f32>>;
@group(2) @binding(5) var<storage, read_write> boidsCellIdDst : array<u32>;

// from iq https://www.shadertoy.com/view/llGSzw
fn hash1(n: u32) -> f32 {
	var n = (n << 13u) ^ n;
  n = n * (n * n * 15731u + 789221u) + 1376312589u;
  return f32(n & u32(0x7fffffffu))/f32(0x7fffffff);
}

fn position_to_grid_cell_id(position: vec2<f32>, grid_size: u32) -> u32 {
  let position_id_f32: vec2<f32> = floor(position * f32(grid_size));
  return grid_size * u32(position_id_f32.y) + u32(position_id_f32.x);
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) GlobalInvocationID : vec3<u32>) {
  let total = arrayLength(&boidsPositionDst);
  let index: u32 = GlobalInvocationID.x;
  if (index >= total) { return; }

  let alterated_index: u32 = index * 142857u + initParameters.seed;

  // Init boid with random velocity and position
  boidsPositionDst[index] = vec2<f32>(hash1(alterated_index), hash1(alterated_index + 1u));
  boidsVelocityDst[index] = normalize(vec2<f32>(hash1(alterated_index + 2u), hash1(alterated_index + 3u)) * 2.0 - 1.0)* 0.01;
  boidsCellIdDst[index] = position_to_grid_cell_id(boidsPositionDst[index], simulationParameters.grid_size);
}