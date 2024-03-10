struct FlockingParameters {
    avgPosition: vec2<f32>,
    avoidance: vec2<f32>,
    avgVelocity: vec2<f32>,
    neighborCount: u32,
    avoidCount: u32,
}

// detla time
const detlaTime: f32 = 0.0166;

fn flockingInit() -> FlockingParameters {
    return FlockingParameters(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        0u,
        0u
    );
}

fn flockingAccumulate(
    currentPosition: vec2<f32>,
    currentVelocity: vec2<f32>,
    otherPosition: vec2<f32>,
    otherVelocity: vec2<f32>,
    flockingParameters: ptr<function, FlockingParameters>,
    ) {

    let current_to_other = otherPosition - currentPosition;
    let sqrt_distance = dot(current_to_other, current_to_other);

    let sqrt_view_radius = simulationParameters.view_radius * simulationParameters.view_radius;    
    // Skip if too far away
    if (sqrt_distance > sqrt_view_radius) {
        return;
    }

    // Visiblity angle
    if (dot(normalize(current_to_other), normalize(currentVelocity)) < -0.6) {
        return;
    }
    
    // Separation
    if (sqrt_distance < sqrt_view_radius * simulationParameters.separation_radius_factor * simulationParameters.separation_radius_factor) {
        (*flockingParameters).avoidance -= current_to_other;
        (*flockingParameters).avoidCount += 1u;
    }

    (*flockingParameters).avgVelocity += otherVelocity; // Aligment
    (*flockingParameters).avgPosition += otherPosition; // Cohesion
    (*flockingParameters).neighborCount += 1u;
}

fn flockingPostAccumulation(
    flockingParameters: ptr<function, FlockingParameters>
    ) {
    if ((*flockingParameters).neighborCount > 0u) {
        var f32_neighborCount = f32((*flockingParameters).neighborCount);
        (*flockingParameters).avgPosition /= f32_neighborCount;
        (*flockingParameters).avgVelocity /= f32_neighborCount;
    }
}

fn computeNewVelocity(
    currentPosition: vec2<f32>,
    currentVelocity: vec2<f32>,
    flockingParameters: FlockingParameters,
    ) -> vec2<f32> {

    var acceleration : vec2<f32> = vec2<f32>(0.0, 0.0);

    // Todo: make this a parameter
    var max_speed : f32 = 0.1;
    var min_speed : f32 = 0.01;
    var max_steering_strength : f32 = max_speed * 2.0;

    if (flockingParameters.neighborCount > 0u) {
        acceleration += normalize(flockingParameters.avgVelocity)  * simulationParameters.aligment_scale;
        acceleration += normalize(flockingParameters.avgPosition - currentPosition) * simulationParameters.cohesion_scale;

        if(flockingParameters.avoidCount > 0u) {
            acceleration += normalize(flockingParameters.avoidance) * simulationParameters.separation_scale;
        }
    }

    acceleration += edge_repulsion(currentPosition, currentVelocity, simulationParameters.repulsion_margin/2.0, simulationParameters.repulsion_strength);
    
    var vel = currentVelocity + acceleration * detlaTime;

    let vel_mag = length(currentVelocity);

    // Limit speed
    vel = (vel / vel_mag) * clamp(vel_mag, min_speed, max_speed);
    return vel;
}

fn computeNewPosition(
    currentPosition: vec2<f32>,
    currentVelocity: vec2<f32>,
    ) -> vec2<f32> {
    return currentPosition + currentVelocity * detlaTime;
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
