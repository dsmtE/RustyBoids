struct BoidData {
    @location(0) position: vec2<f32>,
    @location(1) velocity: vec2<f32>,
    @location(2) current_cell_id: vec2<u32>,
};

struct SimulationParameters {
    delta_t: f32,
    view_radius: f32,
    cohesion_scale: f32,
    aligment_scale: f32,
    separation_scale: f32,
    grid_count: u32,
}

@group(0) @binding(0) var<uniform> simulationParameters : SimulationParameters;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) current_cell_id : u32,
};

@vertex
fn vs_main(
    boid: BoidData,
    @location(3) position: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;

    let angle = -atan2(boid.velocity.x, boid.velocity.y);
    let c = cos(angle);
    let s = sin(angle);

    let scale = 0.3;
    var pos = vec2<f32>(
        position.x * c - position.y * s,
        position.x * s + position.y * c,
    );
    pos *= scale;
    
    // shift to displat boid in [0, 1] in range of the screen [-1, 1]
    let centered_boid = boid.position * 2.0 - 1.0;

    out.clip_position = vec4<f32>(pos.x + centered_boid.x, pos.y + centered_boid.y, 0.0, 1.0);
    out.current_cell_id = boid.current_cell_id.x;
    return out;
}

// from iq : https://iquilezles.org/articles/palettes/
fn palette(t: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    return a + b * cos(6.28318 * (c * t + d));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let grid_count_f32 = f32(simulationParameters.grid_count);
    let cell_factor = f32(in.current_cell_id) / (grid_count_f32*grid_count_f32);
    let color: vec3<f32> = palette(cell_factor, vec3<f32>(0.2,0.2,0.2),vec3<f32>(0.8,0.8,0.8),vec3<f32>(1.0,1.0,1.0)*14.2857,vec3<f32>(0.0,0.33,0.67));
    return vec4<f32>(color.x, color.y, color.z, 1.0);
}
 
 