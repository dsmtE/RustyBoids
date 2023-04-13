struct BoidData {
  position: vec2<f32>,
  velocity: vec2<f32>,
  current_cell_id: vec2<u32>,
};

struct SortingParams {
  h: u32,
};

var<workgroup> local_boids : array<BoidData, 512>;

@group(0) @binding(0) var<uniform> sortingParams : SortingParams;

// @group(1) @binding(0) var<storage, read> boidsSrc : array<BoidData>;
@group(1) @binding(1) var<storage, read_write> boidsDst : array<BoidData>;

fn local_compare_and_swap(a: u32, b: u32) {
	if local_boids[a].current_cell_id.x > local_boids[b].current_cell_id.x {
		let temp = local_boids[a];
    local_boids[a] = local_boids[b];
    local_boids[b] = temp;
	}
}

fn global_compare_and_swap(a: u32, b: u32) {
	if boidsDst[a].current_cell_id.x > boidsDst[b].current_cell_id.x {
		let temp = boidsDst[a];
    boidsDst[a] = boidsDst[b];
    boidsDst[b] = temp;
	}
}

fn local_flip(h: u32, local_id: u32, workgroup_id: u32) {
	  let half_h = h / 2u;
    let q = ((2u * local_id) / h) * h;
    let m = (local_id % half_h);
    local_compare_and_swap(
      q + m,
      q + h - m - 1u
    );
}

fn big_flip(h: u32, global_id: u32) {
	let half_h = h / 2u;
  let q = ((2u * global_id) / h) * h;
  let m = (global_id % half_h);
  global_compare_and_swap(
    q + m,
    q + h - m - 1u
  );
}

fn local_disperse(h: u32, local_id: u32) {
	let half_h = h / 2u;
  let x = ((2u * local_id) / h) * h + (local_id % half_h);
  local_compare_and_swap(
    x,
    x + half_h
  );
}

fn big_disperse(h: u32, global_id: u32) {
	let half_h = h / 2u;
  let x = ((2u * global_id) / h) * h + (global_id % half_h);
  global_compare_and_swap(
    x,
    x + half_h
  );
}

fn local_bitonic_merge_sort(h: u32, local_id: u32, workgroup_id: u32) {
	var hh = 2u;
	while hh <= h {
		local_flip(hh, local_id, workgroup_id);
    workgroupBarrier();
		var hhh = hh / 2u;
		while hhh > 1u {
			local_disperse(hhh, local_id);
      workgroupBarrier();
			hhh /= 2u;
		}
		hh *= 2u;
	}
}

fn copy_to_local_memory(local_id: u32, workgroup_id: u32) {
  // Calculate global offset for local workgroup
  let offset: u32 = workgroup_size_x * 2u * workgroup_id;

  // copy to local memory
  local_boids[local_id*2u]   = boidsDst[offset+local_id*2u];
  local_boids[local_id*2u+1u] = boidsDst[offset+local_id*2u+1u];
  workgroupBarrier();
}

fn copy_to_global_memory(local_id: u32, workgroup_id: u32) {
  // Calculate global offset for local workgroup
  let offset: u32 = workgroup_size_x * 2u * workgroup_id;

  // copy back to global memory
  boidsDst[offset+local_id*2u]   = local_boids [local_id*2u];
  boidsDst[offset+local_id*2u+1u] = local_boids [local_id*2u+1u];
  workgroupBarrier();
}

const workgroup_size_x: u32 = 64u;

// TODO: make it work unsing workgroup_size_x (@workgroup_size(workgroup_size_x))
@compute @workgroup_size(64)
fn cs_local_bms(
  @builtin(workgroup_id) WorkGroupID : vec3<u32>,
  @builtin(local_invocation_id) LocalInvocationID : vec3<u32>,
  @builtin(global_invocation_id) GlobalInvocationID : vec3<u32>
) {
  let total = arrayLength(&boidsDst);
  let global_id = GlobalInvocationID.x;
  if (global_id >= total) { return; }

  let local_id = LocalInvocationID.x;
  let workgroup_id = WorkGroupID.x;

  copy_to_local_memory(local_id, workgroup_id);

  local_bitonic_merge_sort(sortingParams.h, local_id, workgroup_id);
  
  copy_to_global_memory(local_id, workgroup_id);
}

@compute @workgroup_size(64)
fn cs_big_flip(
  @builtin(workgroup_id) WorkGroupID : vec3<u32>,
  @builtin(local_invocation_id) LocalInvocationID : vec3<u32>,
  @builtin(global_invocation_id) GlobalInvocationID : vec3<u32>
) {
  let total = arrayLength(&boidsDst);
  let global_id = GlobalInvocationID.x;
  if (global_id >= total) { return; }

  big_flip(sortingParams.h, global_id);
}

@compute @workgroup_size(64)
fn cs_big_disperse(
  @builtin(workgroup_id) WorkGroupID : vec3<u32>,
  @builtin(local_invocation_id) LocalInvocationID : vec3<u32>,
  @builtin(global_invocation_id) GlobalInvocationID : vec3<u32>
) {
  let total = arrayLength(&boidsDst);
  let global_id = GlobalInvocationID.x;
  if (global_id >= total) { return; }

  big_disperse(sortingParams.h, global_id);
}

@compute @workgroup_size(64)
fn cs_local_disperse(
  @builtin(workgroup_id) WorkGroupID : vec3<u32>,
  @builtin(local_invocation_id) LocalInvocationID : vec3<u32>,
  @builtin(global_invocation_id) GlobalInvocationID : vec3<u32>
) {
  let total = arrayLength(&boidsDst);
  let global_id = GlobalInvocationID.x;
  if (global_id >= total) { return; }

  let local_id = LocalInvocationID.x;
  let workgroup_id = WorkGroupID.x;

  copy_to_local_memory(local_id, workgroup_id);

  local_disperse(sortingParams.h, local_id);
  
  copy_to_global_memory(local_id, workgroup_id);
}

