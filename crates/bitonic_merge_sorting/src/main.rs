
// This is a CPU Rust implementation of the bitonic merge sort algorithm.

// Inspired from implementation well described in this article https://poniesandlight.co.uk/reflect/bitonic_merge_sort/
use clap::Parser;
use rand::thread_rng;
use rand::seq::SliceRandom;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
	#[arg(short, long, help("number of elements to sort"), default_value_t = 16)]
    number: usize,
	#[arg(short, long, help("how much data is available in shader local memory to store sortable elements."), default_value_t = 16)]
	max_workgroup_size: usize,
}

// Argument validation
impl Arguments {
	fn parse_and_validate() -> Self {
		let args = Self::parse();
		assert!(args.number > 2, "number must be > 2.");
		assert!(args.number % 2 == 0, "number must be a power of 2.");
		args
	}
}

fn main() {

	let args = Arguments::parse_and_validate();
    println!("{:?}", args);

	// destructuring
	let Arguments { number: n, max_workgroup_size } = args;

	let workgroup_size_x = if n < max_workgroup_size * 2 { n / 2 } else { max_workgroup_size };

    let mut vec: Vec<u32> = (0..n as u32).collect();
    vec.shuffle(&mut thread_rng());

	println!("Unsorted: {:?}", vec);

	global_bitonic_merge_sort(&mut vec, workgroup_size_x);

	println!("Sorted: {:?}", vec);

	assert!(vec.windows(2).all(|w| w[0] <= w[1]));
}

fn local_compare_and_swap(vec: &mut Vec<u32>, a: usize, b: usize) {
	print!("[{},{}]", a, b);
	if vec[a] > vec[b] {
		vec.swap(a, b)
	}
}

// here not real difference between local and global compare and swap
// This will be used in the future when we will have a global memory and a local memory
fn global_compare_and_swap(vec: &mut Vec<u32>, a: usize, b: usize) {
	print!("[{}, {}] ", a, b);
	if vec[a] > vec[b] {
		vec.swap(a, b)
	}
}

fn local_flip(vec: &mut Vec<u32>, h:usize, workgroup_size_x: usize) {
	print!("Local flip over height {} ", h);
	let n = vec.len();
	let workgroup_count = n / ( workgroup_size_x * 2 );
	let half_h = h / 2;

	// Simulate workgroups and barrier using loops
	for workgroup_id in 0..workgroup_count {
		print!(" |wg({})| ", workgroup_id);
		let h_offset = h * ( ( workgroup_size_x * workgroup_id * 2 ) / h );

		for local_thread_id in 0..workgroup_size_x {
			let q = h_offset + ((2 * local_thread_id) / h) * h;
			
			local_compare_and_swap(
				vec,
				q + local_thread_id % half_h,
				q + h - (local_thread_id % half_h) - 1
			);
		}
	}
	println!("");
}

fn big_flip(vec: &mut Vec<u32>, h: usize, workgroup_size_x: usize) {
	print!("Big Flip over height {} ", h);

	let n = vec.len();
	let workgroup_count = n / ( workgroup_size_x * 2 );
	let half_h = h / 2;

	// TODO: rewrite using macro to avoid code duplication
	// Simulate workgroups and barrier using loops
	for workgroup_id in 0..workgroup_count {
		print!(" |wg({})| ", workgroup_id);
		for local_thread_id in 0..workgroup_size_x {
			
			let global_thread_id = workgroup_id * workgroup_size_x + local_thread_id;
			let q = ( ( 2 * global_thread_id ) / h ) * h;

			global_compare_and_swap(
				vec,
				q + ( global_thread_id % half_h ),
				q + h - ( global_thread_id % half_h ) - 1
			);
		}
	}
	println!("");
}

fn local_disperse(vec: &mut Vec<u32>, h: usize, workgroup_size_x: usize) {
	print!("local Disperse over height {} ", h);
	let n = vec.len();
	let workgroup_count = n / ( workgroup_size_x * 2 );
	let half_h = h / 2;

	for workgroup_id in 0..workgroup_count {
		print!(" |wg({})| ", workgroup_id);
		let h_offset = h * ( ( workgroup_size_x * workgroup_id * 2 ) / h );

		for local_thread_id in 0..workgroup_size_x {
			let q = h_offset + ((2 * local_thread_id) / h) * h;

			let x = q + local_thread_id % half_h;
			local_compare_and_swap(vec,
				x,
				x + half_h
			);
		}
	}
	println!("");
}

fn big_disperse(vec: &mut Vec<u32>, h: usize, workgroup_size_x: usize) {
	print!("Big Disperse over height {} | ", h);
	//assert!(workgroup_size_x * 2 > h, "number of sortable elements processed by one workgroup must be smaller or equal to flip height");
	let n = vec.len();
	let workgroup_count = n / ( workgroup_size_x * 2 );
	let half_h = h / 2;

	for workgroup_id in 0..workgroup_count {
		print!(" |wg({})| ", workgroup_id);
		for local_thread_id in 0..workgroup_size_x {
			let global_thread_id = workgroup_id * workgroup_size_x + local_thread_id;
			let q = ( ( 2 * global_thread_id ) / h ) * h;

			let x = q + (global_thread_id % half_h);
			global_compare_and_swap(vec,
				x,
				x + half_h
			);
		}
	}
	println!("");
}

// Use vec here as shared memory
fn local_bitonic_merge_sort(vec: &mut Vec<u32>, h: usize, workgroup_size_x: usize) {
	println!("local BMS height: {} Workgroup size: {}", h, workgroup_size_x);

	let mut hh = 2;
	while hh <= h {
		local_flip(vec, hh, workgroup_size_x);
		let mut hhh = hh / 2;
		while hhh > 1 {
			local_disperse(vec, hhh, workgroup_size_x);
			hhh /= 2;
		}
		hh *= 2;
	}
}

fn global_bitonic_merge_sort(vec: &mut Vec<u32>, workgroup_size_x: usize) {
	let n = vec.len();
	println!("global BMS {} elements, Workgroup size: {}", n, workgroup_size_x);
	let mut h = workgroup_size_x * 2;

	local_bitonic_merge_sort(vec, h, workgroup_size_x);

	// we must now double h, as this happens before every flip
	h *= 2;
	while h <= n {
		big_flip( vec, h, workgroup_size_x);

		let mut hh = h / 2;
		
		while hh > 1 {
			if hh <= workgroup_size_x * 2 {
				local_disperse(vec, hh, workgroup_size_x);
			} else {
				big_disperse(vec, hh, workgroup_size_x);
			}
			hh /= 2;
		}

		h *= 2;
	}
}
