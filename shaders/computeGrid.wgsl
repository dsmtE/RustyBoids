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

@group(0) @binding(0) var<uniform> simulationParameters : SimulationParameters;

@group(1) @binding(0) var<storage, read> boidsPositionSrc : array<vec2<f32>>;
@group(1) @binding(1) var<storage, read> boidsVelocitySrc : array<vec2<f32>>;
@group(1) @binding(2) var<storage, read> boidsCellIdSrc : array<u32>;

@group(1) @binding(3) var<storage, read_write> boidsPositionDst : array<vec2<f32>>;
@group(1) @binding(4) var<storage, read_write> boidsVelocityDst : array<vec2<f32>>;
@group(1) @binding(5) var<storage, read_write> boidsCellIdDst : array<u32>;

// boid_sorting_id
@group(2) @binding(0) var<storage, read> sorting_id : array<u32>;
@group(3) @binding(0) var<storage, read> cell_count_partial_sum : array<u32>;

//!include flocking.wgsl

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) GlobalInvocationID : vec3<u32>) {
  let total = arrayLength(&boidsPositionSrc);
  let index = GlobalInvocationID.x;
  if (index >= total) { return; }

  var currentPosition : vec2<f32> = boidsPositionSrc[sorting_id[index]];
  var currentVelocity : vec2<f32> = boidsVelocitySrc[sorting_id[index]];
  var currentCellId : u32 = boidsCellIdSrc[sorting_id[index]];

  // Flocking
  var flockingParameters = flockingInit();

  // Accumulate over neighbors using cell_count_partial_sum and neighbor cells (8)
  // for loops for each ligne of the grid (2D case)
  for (var i : u32 = 0u; i < 3u; i = i + 1u) {

    // split cell id into x and y
    var cell_id_x : u32 = currentCellId % simulationParameters.grid_size;
    var cell_id_y : u32 = currentCellId / simulationParameters.grid_size;

    //skip down and up lines
    if (
      (i == 0u && cell_id_y == 0u) ||
      (i == 2u && cell_id_y == simulationParameters.grid_size - 1u)
    ) { continue; }

    var line_offset_cell_id : u32 = currentCellId + (i - 1u) * simulationParameters.grid_size;
    // Take care of edge cases using min on x
    var begin_range_cell_id : u32 = line_offset_cell_id - min(cell_id_x, 1u);
    var end_range_cell_id : u32 = line_offset_cell_id  + min((simulationParameters.grid_size - 1u) - cell_id_x, 1u);

    var begin_range_id : u32 = cell_count_partial_sum[begin_range_cell_id];
    var end_range_id : u32 = cell_count_partial_sum[end_range_cell_id+1u] - 1u;

    // Use cell_count_partial_sum to iterate over the neighbors range on the given line
    for (var j : u32 = begin_range_id; j <= end_range_id; j = j + 1u) {
      if (j == index) { continue; }
      flockingAccumulate(currentPosition, currentVelocity, j, &flockingParameters);
    }
  }

  flockingPostAccumulation(&flockingParameters);

  // Update velocity
  var newVelocity : vec2<f32> = computeNewVelocity(currentPosition, currentVelocity, flockingParameters, simulationParameters);
  var newPosition : vec2<f32> = computeNewPosition(currentPosition, newVelocity);

  // Write back to storage buffer
  // no mater if we use boid_sorting_id as this will be sorted again
  boidsPositionDst[index] = newPosition;
  boidsVelocityDst[index] = newVelocity;
  boidsCellIdDst[index] = position_to_grid_cell_id(newPosition, simulationParameters.grid_size);
}

fn position_to_grid_cell_id(position: vec2<f32>, grid_size: u32) -> u32 {
  let position_id_f32: vec2<f32> = floor(position * f32(grid_size));
  return (grid_size * u32(position_id_f32.y) + u32(position_id_f32.x)) % (grid_size * grid_size);
}
