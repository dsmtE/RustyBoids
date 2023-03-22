
// This is a Rust implementation of the bitonic merge sort algorithm.

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

// Arguyment validation
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

	bitonic_merge_sort(&mut vec, workgroup_size_x);

	println!("Sorted: {:?}", vec);

	assert!(vec.windows(2).all(|w| w[0] <= w[1]));
}

fn local_compare_and_swap(vec: &mut Vec<u32>, a: usize, b: usize) {
	print!("[{}, {}] ", a, b);
	if vec[a] > vec[b] {
		vec.swap(a, b)
	}
}

fn do_flip(vec: &mut Vec<u32>, t:usize, h: usize) {
	let q = ((2 * t) / h) * h;
	let half_h = h / 2;
	local_compare_and_swap(
		vec,
		q + t % half_h,
		q + h - (t % half_h) - 1
	);
}

fn do_disperse(vec: &mut Vec<u32>, t: usize, h: usize) {
	let q = ((2 * t) / h) * h;
	let half_h = h / 2;
	local_compare_and_swap(vec,
		q + t % half_h,
		q + (t % half_h) + half_h
	);
}

// Use vec here as shared memory
fn bitonic_merge_sort(vec: &mut Vec<u32>, workgroup_size_x: usize) {
	let mut h = 2;
	let n = vec.len();

	println!("{} elements, Workgroup size: {}", n, workgroup_size_x);
	while h <= n {

		// this loop normally would be a barrier
		print!("Flip over height {} | ", h);
		for t in 0..workgroup_size_x {
			do_flip(vec, t, h);
		}
		println!("");

		let mut hh = h / 2;
		while hh > 1 {

			// this loop normally would be a barrier
			print!("Disperse over height {} | ", hh);
			for t in 0..workgroup_size_x {
				do_disperse(vec, t, hh);
			}
			println!("");
			
			hh /= 2;
		}
		h *= 2;
	}

}
