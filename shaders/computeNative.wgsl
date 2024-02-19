struct SimulationParameters {
  view_radius: f32,
  separation_radius_factor: f32,
  cohesion_scale: f32,
  aligment_scale: f32,
  separation_scale: f32,
  repulsion_margin: f32,
  repulsion_strength: f32,
  boids_count: u32,
  grid_size: u32,
}

@group(0) @binding(0) var<uniform> simulationParameters : SimulationParameters;

@group(1) @binding(0) var<storage, read> boidsPositionSrc : array<vec2<f32>>;
@group(1) @binding(1) var<storage, read> boidsVelocitySrc : array<vec2<f32>>;

@group(1) @binding(3) var<storage, read_write> boidsPositionDst : array<vec2<f32>>;
@group(1) @binding(4) var<storage, read_write> boidsVelocityDst : array<vec2<f32>>;

//!include flocking.wgsl

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) GlobalInvocationID : vec3<u32>) {

  let total = arrayLength(&boidsPositionSrc);
  let index = GlobalInvocationID.x;
  if (index >= total) { return; }

  var currentPosition : vec2<f32> = boidsPositionSrc[index];
  var currentVelocity : vec2<f32> = boidsVelocitySrc[index];

  var flockingParameters = flockingInit();

  for (var i : u32 = 0u; i < total; i = i + 1u) {
    if (i == index) { continue; } // skip self
    flockingAccumulate(currentPosition, currentVelocity, boidsPositionSrc[i], boidsVelocitySrc[i], &flockingParameters);
  }

  flockingPostAccumulation(&flockingParameters);

  // Update velocity
  var newVelocity : vec2<f32> = computeNewVelocity(currentPosition, currentVelocity, flockingParameters);
  var newPosition : vec2<f32> = computeNewPosition(currentPosition, newVelocity);

  // Write back to storage buffer
  // no mater if we use boid_sorting_id as this will be sorted again
  boidsPositionDst[index] = newPosition;
  boidsVelocityDst[index] = newVelocity;
}
