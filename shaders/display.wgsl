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
@group(1) @binding(2) var<storage, read> boidsCellIdSrc : array<u32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) boid_sorting_id: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    var boid_position = boidsPositionSrc[boid_sorting_id];
    var boid_velocity = boidsVelocitySrc[boid_sorting_id];
    var boid_cell_id = boidsCellIdSrc[boid_sorting_id];

    let angle = -atan2(boid_velocity.x, boid_velocity.y);
    let c = cos(angle);
    let s = sin(angle);

    let scale = 0.3;
    var pos = vec2<f32>(
        position.x * c - position.y * s,
        position.x * s + position.y * c,
    );
    pos *= scale;
    
    // shift to displat boid in [0, 1] in range of the screen [-1, 1]
    let centered_boid = boid_position * 2.0 - 1.0;

    out.clip_position = vec4<f32>(pos.x + centered_boid.x, pos.y + centered_boid.y, 0.0, 1.0);
    // let color_factor = cell_factor(boid_cell_id, simulationParameters.grid_size);
    let color_factor = f32(instance_index) / f32(simulationParameters.boids_count);
    out.color = palette(color_factor, vec3<f32>(0.2,0.2,0.2),vec3<f32>(0.8,0.8,0.8),vec3<f32>(1.0,1.0,1.0),vec3<f32>(0.0,0.33,0.67));
    return out;
}

// from iq : https://iquilezles.org/articles/palettes/
fn palette(t: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    return a + b * cos(6.28318 * (c * t + d));
}

fn cell_factor(cell_id: u32, grid_size: u32) -> f32 {
    let grid_size_f32 = f32(grid_size);
    return f32(cell_id) / (grid_size_f32*grid_size_f32);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color.x, in.color.y, in.color.z, 1.0);
}