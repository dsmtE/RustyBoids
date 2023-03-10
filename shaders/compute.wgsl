struct BoidData {
    position: vec2<f32>,
    velocity: vec2<f32>,
};

struct SimulationParameters {
    delta_t: f32,
    cohesion_distance: f32,
    aligment_distance: f32,
    separation_distance: f32,
    cohesion_scale: f32,
    aligment_scale: f32,
    separation_scale: f32,
}

struct Boids {
  boids : array<BoidData>,
}

@group(0) @binding(0) var<uniform> simulationParameters : SimulationParameters;
@group(1) @binding(0) var<storage, read> boidsSrc : Boids;
@group(1) @binding(1) var<storage, read_write> boidsDst : Boids;

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
    // Separation
    if (distance < simulationParameters.separation_distance) {
      close = close + (currentPosition - otherPosition);
    }

    // Aligment
    if (distance < simulationParameters.aligment_distance) {
      avgVelocity = avgVelocity + otherVelocity;
      avgVelocityCount = avgVelocityCount + 1u;
    }

    // Cohesion
    if (distance < simulationParameters.cohesion_distance) {
      avgPosition = avgPosition + otherPosition;
      avgPositionCount = avgPositionCount + 1u;
    }
  }

  if (avgPositionCount > 0u) {
    avgPosition = avgPosition / f32(avgPositionCount);
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
}

fn wrap_arroud(v : vec2<f32>) -> vec2<f32> {
  var result : vec2<f32> = v;
  if (v.x < -1.0) { result.x = 1.0; }
  if (v.x > 1.0) { result.x = -1.0; }
  if (v.y < -1.0) { result.y = 1.0; }
  if (v.y > 1.0) { result.y = -1.0; }
  return result;
}