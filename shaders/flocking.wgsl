struct FlockingParameters {
    avgPosition: vec2<f32>,
    close: vec2<f32>,
    avgVelocity: vec2<f32>,
    neighborCount: u32,
}

fn flockingInit() -> FlockingParameters {
    return FlockingParameters(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
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

    let otherToCurrent = currentPosition - otherPosition;

    let sqrt_distance = dot(otherToCurrent, otherToCurrent);
    let sqrt_view_radius = simulationParameters.view_radius * simulationParameters.view_radius;

    // Skip if too far away
    if (sqrt_distance > sqrt_view_radius) {
        return;
    }

    // Visiblity angle
    if (dot(normalize(-otherToCurrent), normalize(currentVelocity)) < -0.5) {
        return;
    }
    
    // Separation
    if (sqrt_distance <= sqrt_view_radius * simulationParameters.separation_radius_factor * simulationParameters.separation_radius_factor) {
        let separation_distance = simulationParameters.view_radius * simulationParameters.separation_radius_factor;
        (*flockingParameters).close = (*flockingParameters).close + otherToCurrent / sqrt_distance * separation_distance;
    }

    // Aligment
    (*flockingParameters).avgVelocity = (*flockingParameters).avgVelocity + otherVelocity;

    // Cohesion
    (*flockingParameters).avgPosition = (*flockingParameters).avgPosition + otherPosition;

    (*flockingParameters).neighborCount = (*flockingParameters).neighborCount + 1u;
}

fn flockingPostAccumulation(
    flockingParameters: ptr<function, FlockingParameters>
    ) {
    if ((*flockingParameters).neighborCount > 0u) {
        var f32_neighborCount = f32((*flockingParameters).neighborCount);
        (*flockingParameters).avgPosition = (*flockingParameters).avgPosition / f32_neighborCount;
        (*flockingParameters).avgVelocity = (*flockingParameters).avgVelocity / f32_neighborCount;
    }
}

fn computeNewVelocity(
    currentPosition: vec2<f32>,
    currentVelocity: vec2<f32>,
    flockingParameters: FlockingParameters,
    ) -> vec2<f32> {

    var acceleration : vec2<f32> = vec2<f32>(0.0, 0.0);

    // Todo: make this a parameter
    var maxVelocity : f32 = 0.08;

    if (flockingParameters.neighborCount > 0u) {
        // acceleration += (flockingParameters.avgVelocity - currentVelocity) * simulationParameters.aligment_scale;
        acceleration += (normalize(flockingParameters.avgVelocity) - normalize(currentVelocity)) * simulationParameters.aligment_scale;
        acceleration += (flockingParameters.avgPosition - currentPosition) * simulationParameters.cohesion_scale;
        acceleration += flockingParameters.close * simulationParameters.separation_scale;
        acceleration *= maxVelocity;
    }

    acceleration += edge_repulsion(currentPosition, currentVelocity, simulationParameters.repulsion_margin/2.0, simulationParameters.repulsion_strength);
    
    var newVelocity : vec2<f32> = currentVelocity + acceleration;

    // Clamp velocity for a more pleasing simulation
    newVelocity = clamp_to_max(newVelocity, maxVelocity);

    return newVelocity;
}

fn computeNewPosition(
    currentPosition: vec2<f32>,
    currentVelocity: vec2<f32>,
    ) -> vec2<f32> {
    var newPosition : vec2<f32> = currentPosition + currentVelocity;
    // return wrap_arroud(newPosition);
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

fn clamp_to_max(v: vec2<f32>, max_magnitude: f32) -> vec2<f32> {
    var magnitude = length(v);
    if (magnitude > max_magnitude) {
        return v / magnitude * max_magnitude;
    }
    return v;
}
