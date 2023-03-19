struct BoidData {
    @location(0) position: vec2<f32>,
    @location(1) velocity: vec2<f32>,
    @location(2) current_cell_id: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
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
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}
 
 