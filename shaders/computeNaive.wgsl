struct BoidData {
    position: vec2<f32>,
    velocity: vec2<f32>,
    current_cell_id: vec2<u32>,
};

struct Boids {
  boids : array<BoidData>,
}

struct SimulationParameters {
    delta_t: f32,
    view_radius: f32,
    cohesion_scale: f32,
    aligment_scale: f32,
    separation_scale: f32,
    grid_count: u32,
}

@group(0) @binding(0) var<uniform> simulationParameters : SimulationParameters;
@group(1) @binding(0) var<storage, read> boidsSrc : Boids;
@group(1) @binding(1) var<storage, read_write> boidsDst : Boids;

fn position_to_grid_cell_id(position: vec2<f32>, grid_count: u32) -> u32 {
  let position_id_f32: vec2<f32> = floor(position * f32(grid_count));
  return grid_count * u32(position_id_f32.y) + u32(position_id_f32.x);
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) GlobalInvocationID : vec3<u32>) {
  let total = arrayLength(&boidsSrc.boids);
  let index = GlobalInvocationID.x;
  if (index >= total) { return; }

  var currentPosition : vec2<f32> = boidsSrc.boids[index].position;
  var currentVelocity : vec2<f32> = boidsSrc.boids[index].velocity;

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

    let otherPosition = boidsSrc.boids[i].position;
    let otherVelocity = boidsSrc.boids[i].velocity;

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
  newVelocity = normalize(newVelocity) * clamp(length(newVelocity), 0.0, 0.1);

  // Update position
  var newPosition : vec2<f32> = currentPosition + newVelocity * simulationParameters.delta_t;

  // Wrap around boundary
  newPosition = wrap_arroud(newPosition);

  // Write back to storage buffer
  boidsDst.boids[index].position = newPosition;
  boidsDst.boids[index].velocity = newVelocity;
  boidsDst.boids[index].current_cell_id.x = position_to_grid_cell_id(newPosition, simulationParameters.grid_count);
}

fn wrap_arroud(v : vec2<f32>) -> vec2<f32> {
  var result : vec2<f32> = v;
  if (v.x < 0.0) { result.x = 1.0; }
  if (v.x > 1.0) { result.x = 0.0; }
  if (v.y < 0.0) { result.y = 1.0; }
  if (v.y > 1.0) { result.y = 0.0; }
  return result;
}