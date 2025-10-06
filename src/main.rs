#[cfg(target_os = "hermit")]
use hermit as _;

use alloc_benches::{rayon_boxes, Params};
#[allow(unused_imports)]
use alloc_benches::{KB, MB, GB, TB};

use rayon::ThreadPoolBuilder;

fn main() {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let params = Params {
        keep_max: 128,
        keep_prob: 0.01,
        max_size: 1 * MB,
        iters_per_task: 10_000,
        tasks: 2 * threads,
        threads,
    };
    let pool = ThreadPoolBuilder::new()
        .num_threads(params.threads)
        .build()
        .unwrap();
    rayon_boxes(&pool, params);

    println!("done");
}
