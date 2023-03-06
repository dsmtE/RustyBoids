struct BoidData {
    @location(0) position: vec2<f32>,
    @location(1) velocity: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    boid: BoidData,
    @location(2) position: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;

    let angle = -atan2(boid.velocity.y, boid.velocity.x);
    let c = cos(angle);
    let s = sin(angle);
    let pos = vec2(
        position.x * c - position.y * s,
        position.x * s + position.y * c,
    );

    out.clip_position = vec4<f32>(pos.x + boid.position.x, pos.y + boid.position.y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}
 
 