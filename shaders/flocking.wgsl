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

    // Cohesion
    (*flockingParameters).avgPosition = (*flockingParameters).avgPosition + otherPosition;

    (*flockingParameters).neighborCount = (*flockingParameters).neighborCount + 1u;
}

fn flockingPostAccumulation(
    flockingParameters: ptr<function, FlockingParameters>
    ) {
    if ((*flockingParameters).neighborCount > 0u) {
        let f32NeighborCount = f32((*flockingParameters).neighborCount);
        (*flockingParameters).avgPosition = (*flockingParameters).avgPosition / f32NeighborCount;
        (*flockingParameters).avgVelocity = (*flockingParameters).avgVelocity / f32NeighborCount;
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