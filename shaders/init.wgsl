struct BoidData {
    position: vec2<f32>,
    velocity: vec2<f32>,
};

struct Boids {
  boids : array<BoidData>,
}

struct InitParameters {
    seed: u32,
}

@group(0) @binding(0) var<uniform> initParameters : InitParameters;
@group(1) @binding(0) var<storage, read> boidsSrc : Boids;
@group(1) @binding(1) var<storage, read_write> boidsDst : Boids;

// from iq https://www.shadertoy.com/view/llGSzw
fn hash1(n: u32) -> f32 {
	var n = (n << 13u) ^ n;
  n = n * (n * n * 15731u + 789221u) + 1376312589u;
  return f32(n & u32(0x7fffffffu))/f32(0x7fffffff);
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) GlobalInvocationID : vec3<u32>) {
  let total = arrayLength(&boidsSrc.boids);
  let index: u32 = GlobalInvocationID.x;
  if (index >= total) { return; }

  let alterated_index: u32 = index * 142857u + initParameters.seed;

  // Init boid with random velocity and position
  boidsDst.boids[index].position = vec2<f32>(hash1(alterated_index), hash1(alterated_index + 1u));
  boidsDst.boids[index].velocity = normalize(vec2<f32>(hash1(alterated_index + 2u), hash1(alterated_index + 3u)) * 2.0 - 1.0)* 0.01;
}