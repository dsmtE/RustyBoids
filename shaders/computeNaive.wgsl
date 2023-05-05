struct SimulationParameters {
  boids_count: u32,
  delta_t: f32,
  view_radius: f32,
  cohesion_scale: f32,
  aligment_scale: f32,
  separation_scale: f32,
  grid_size: u32,
}

@group(0) @binding(0) var<uniform> simulationParameters : SimulationParameters;

@group(1) @binding(0) var<storage, read> boidsPositionSrc : array<vec2<f32>>;
@group(1) @binding(1) var<storage, read> boidsVelocitySrc : array<vec2<f32>>;
@group(1) @binding(2) var<storage, read> boidsCellIdSrc : array<vec2<u32>>;

@group(1) @binding(3) var<storage, read_write> boidsPositionDst : array<vec2<f32>>;
@group(1) @binding(4) var<storage, read_write> boidsVelocityDst : array<vec2<f32>>;
@group(1) @binding(5) var<storage, read_write> boidsCellIdDst : array<vec2<u32>>;

fn position_to_grid_cell_id(position: vec2<f32>, grid_size: u32) -> u32 {
  let position_id_f32: vec2<f32> = floor(position * f32(grid_size));
  return grid_size * u32(position_id_f32.y) + u32(position_id_f32.x);
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) GlobalInvocationID : vec3<u32>) {
  let total = arrayLength(&boidsPositionSrc);
  let index = GlobalInvocationID.x;
  if (index >= total) { return; }

  var currentPosition : vec2<f32> = boidsPositionSrc[index];
  var currentVelocity : vec2<f32> = boidsVelocitySrc[index];

  // Flocking
  var avgPosition : vec2<f32> = vec2<f32>(0.0, 0.0);
  var avgPositionCount : u32 = 0u;
  var close : vec2<f32> = vec2<f32>(0.0, 0.0);
  var avgVelocity : vec2<f32> = vec2<f32>(0.0, 0.0);
  var avgVelocityCount : u32 = 0u;

  for (var i : u32 = 0u; i < total; i = i + 1u) {
    // skip self
    if (i == index) {
      continue;
    }

    let otherPosition = boidsPositionSrc[i];
    let otherVelocity = boidsVelocitySrc[i];

    let distance = distance(otherPosition, currentPosition);

    // Skip if too far away
    if (distance > simulationParameters.view_radius) {
      continue;
    }
    
    // Separation
    close = close + (currentPosition - otherPosition);

    // Aligment
    avgVelocity = avgVelocity + otherVelocity;
    avgVelocityCount = avgVelocityCount + 1u;

    // Cohesion
    avgPosition = avgPosition + otherPosition;
    avgPositionCount = avgPositionCount + 1u;
  }

  if (avgPositionCount > 0u) {
    avgPosition = avgPosition / f32(avgPositionCount);
  }else {
    avgPosition = currentPosition;
  }

  if (avgVelocityCount > 0u) {
    avgVelocity = avgVelocity / f32(avgVelocityCount);
  }

  // Update velocity
  var newVelocity : vec2<f32> = currentVelocity +
      (avgPosition - currentPosition) * simulationParameters.cohesion_scale +
      close * simulationParameters.separation_scale +
      avgVelocity * simulationParameters.aligment_scale;

  // Clamp velocity for a more pleasing simulation
  if (length(newVelocity) > 0.1) {
    newVelocity = normalize(newVelocity) * 0.1;
  }

  // Update position
  var newPosition : vec2<f32> = currentPosition + newVelocity * simulationParameters.delta_t;

  // Wrap around boundary
  newPosition = wrap_arroud(newPosition);

  // Write back to storage buffer
  boidsPositionDst[index] = newPosition;
  boidsVelocityDst[index] = newVelocity;
  boidsCellIdDst[index].x = position_to_grid_cell_id(newPosition, simulationParameters.grid_size);
}

fn wrap_arroud(v : vec2<f32>) -> vec2<f32> {
  var result : vec2<f32> = v;
  if (v.x < 0.0) { result.x = 1.0; }
  if (v.x > 1.0) { result.x = 0.0; }
  if (v.y < 0.0) { result.y = 1.0; }
  if (v.y > 1.0) { result.y = 0.0; }
  return result;
}