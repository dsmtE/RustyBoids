struct SimulationParameters {
  boids_count: u32,
  delta_t: f32,
  view_radius: f32,
  cohesion_scale: f32,
  aligment_scale: f32,
  separation_scale: f32,
  grid_size: u32,
  repulsion_margin: f32,
  repulsion_strength: f32,
}

struct FlockingParameters {
  avgPosition: vec2<f32>,
  avgPositionCount: u32,
  close: vec2<f32>,
  avgVelocity: vec2<f32>,
  avgVelocityCount: u32,
}

@group(0) @binding(0) var<uniform> simulationParameters : SimulationParameters;

@group(1) @binding(0) var<storage, read> boidsPositionSrc : array<vec2<f32>>;
@group(1) @binding(1) var<storage, read> boidsVelocitySrc : array<vec2<f32>>;

@group(1) @binding(3) var<storage, read_write> boidsPositionDst : array<vec2<f32>>;
@group(1) @binding(4) var<storage, read_write> boidsVelocityDst : array<vec2<f32>>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) GlobalInvocationID : vec3<u32>) {
  let total = arrayLength(&boidsPositionSrc);
  let index = GlobalInvocationID.x;
  if (index >= total) { return; }

  var currentPosition : vec2<f32> = boidsPositionSrc[index];
  var currentVelocity : vec2<f32> = boidsVelocitySrc[index];

  // Flocking
  var flockingParameters : FlockingParameters = FlockingParameters(
    vec2<f32>(0.0, 0.0),
    0u,
    vec2<f32>(0.0, 0.0),
    vec2<f32>(0.0, 0.0),
    0u,
  );

  for (var i : u32 = 0u; i < total; i = i + 1u) {
    if (i == index) { continue; } // skip self
    flockingAccumulate(currentPosition, currentVelocity, boidsPositionSrc[i], boidsVelocitySrc[i], &flockingParameters);
  }

  flockingPostAccumulation(&flockingParameters);

  // Update velocity
  var newVelocity : vec2<f32> = computeNewVelocity(currentPosition, currentVelocity, flockingParameters, simulationParameters);
  var newPosition : vec2<f32> = computeNewPosition(currentPosition, newVelocity);

  // Write back to storage buffer
  // no mater if we use boid_sorting_id as this will be sorted again
  boidsPositionDst[index] = newPosition;
  boidsVelocityDst[index] = newVelocity;
}

fn position_to_grid_cell_id(position: vec2<f32>, grid_size: u32) -> u32 {
  let position_id_f32: vec2<f32> = floor(position * f32(grid_size));
  return (grid_size * u32(position_id_f32.y) + u32(position_id_f32.x)) % (grid_size * grid_size);
}

fn flockingAccumulate(
    currentPosition: vec2<f32>,
    currentVelocity: vec2<f32>,
    otherPosition: vec2<f32>,
    otherVelocity: vec2<f32>,
    flockingParameters: ptr<function, FlockingParameters>
    ) {
    let distance = distance(otherPosition, currentPosition);

    // Skip if too far away
    if (distance > simulationParameters.view_radius) {
      return;
    }
    
    // Separation
    (*flockingParameters).close = (*flockingParameters).close + (currentPosition - otherPosition);

    // Aligment
    (*flockingParameters).avgVelocity = (*flockingParameters).avgVelocity + otherVelocity;
    (*flockingParameters).avgVelocityCount = (*flockingParameters).avgVelocityCount + 1u;

    // Cohesion
    (*flockingParameters).avgPosition = (*flockingParameters).avgPosition + otherPosition;
    (*flockingParameters).avgPositionCount = (*flockingParameters).avgPositionCount + 1u;
}

fn flockingPostAccumulation(
    flockingParameters: ptr<function, FlockingParameters>
    ) {
    if ((*flockingParameters).avgPositionCount > 0u) {
      (*flockingParameters).avgPosition = (*flockingParameters).avgPosition / f32((*flockingParameters).avgPositionCount);
    }

    if ((*flockingParameters).avgVelocityCount > 0u) {
      (*flockingParameters).avgVelocity = (*flockingParameters).avgVelocity / f32((*flockingParameters).avgVelocityCount);
    }
}

fn computeNewVelocity(
    currentPosition: vec2<f32>,
    currentVelocity: vec2<f32>,
    flockingParameters: FlockingParameters,
    simulationParameters: SimulationParameters,
    ) -> vec2<f32> {
    var newVelocity : vec2<f32> = currentVelocity +
        (flockingParameters.avgPosition - currentPosition) * simulationParameters.cohesion_scale +
        flockingParameters.close * simulationParameters.separation_scale +
        flockingParameters.avgVelocity * simulationParameters.aligment_scale;

    newVelocity += edge_repulsion(currentPosition, newVelocity, simulationParameters.repulsion_margin/2.0, simulationParameters.repulsion_strength);

    // Clamp velocity for a more pleasing simulation
    var maxVelocity : f32 = 0.1;
    if (length(newVelocity) > maxVelocity) {
      newVelocity = normalize(newVelocity) * maxVelocity;
    }

    return newVelocity;
}

fn computeNewPosition(
    currentPosition: vec2<f32>,
    currentVelocity: vec2<f32>
    ) -> vec2<f32> {
    var newPosition : vec2<f32> = currentPosition + currentVelocity * simulationParameters.delta_t;

    return newPosition;
}

fn wrap_arroud(v : vec2<f32>) -> vec2<f32> {
  var result : vec2<f32> = v;
  if (v.x < 0.0) { result.x = 1.0; }
  if (v.x > 1.0) { result.x = 0.0; }
  if (v.y < 0.0) { result.y = 1.0; }
  if (v.y > 1.0) { result.y = 0.0; }
  return result;
}

fn edge_repulsion(
    currentPosition: vec2<f32>,
    currentVelocity: vec2<f32>,
    repulsion_margin: f32,
    repulsion_strength: f32,
    ) -> vec2<f32> {
  var edgeRepulsionForce : vec2<f32> = vec2<f32>(0.0, 0.0);
  if (currentPosition.x < repulsion_margin) {
    edgeRepulsionForce.x += (repulsion_margin - currentPosition.x);
  }else if (currentPosition.x > 1.0 - repulsion_margin) {
    edgeRepulsionForce.x = ((1.0 - repulsion_margin) - currentPosition.x);
  }

  if (currentPosition.y < repulsion_margin) {
    edgeRepulsionForce.y = (repulsion_margin - currentPosition.y);
  }else if (currentPosition.y > 1.0 - repulsion_margin) {
    edgeRepulsionForce.y = ((1.0 - repulsion_margin) - currentPosition.y);
  }

  return repulsion_strength * edgeRepulsionForce;
}