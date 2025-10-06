#[cfg(target_os = "hermit")]
use hermit as _;

use rand::{rngs::SmallRng, Rng, SeedableRng};
use rayon::prelude::*;
use std::hint::black_box;

pub const KB: usize = 1 << 10;
pub const MB: usize = KB << 10;
pub const GB: usize = MB << 10;
pub const TB: usize = GB << 10;

#[cfg(feature = "virtual_alloc")]
use virtual_alloc::allocator::VirtualAlloc;

#[cfg(feature = "virtual_alloc")]
#[global_allocator]
pub static ALLOC: VirtualAlloc<128> = VirtualAlloc::new_disabled(7 * GB, 16 * TB);

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub keep_max: usize,
    pub keep_prob: f64,
    pub max_size: usize,
    pub iters_per_task: usize,
    pub tasks: usize,
    pub threads: usize,
}

/// Parallel randomized alloc/free patterns using plain Box<Vec<u8>>.
pub fn rayon_boxes(pool: &rayon::ThreadPool, params: Params) {
    pool.install(|| {
        (0..params.tasks).into_par_iter().for_each(|t| {
            let mut rng = SmallRng::seed_from_u64(0xA110C ^ (t as u64));
            let mut stash: Vec<Box<[u8]>> = Vec::with_capacity(params.keep_max.max(1));

            for _ in 0..params.iters_per_task {
                // allocate a random size
                let sz = rng.random_range(1..=params.max_size);
                let mut b = vec![0u8; sz].into_boxed_slice();
                // write to memory
                if sz >= 8 {
                    let last = sz - 1;
                    b[0] = 1; b[last] = 2;
                }
                // randomly keep some allocations
                if rng.random_bool(params.keep_prob) {
                    stash.push(b);
                    // free older items
                    if stash.len() > params.keep_max {
                        let drop_idx = rng.random_range(0..stash.len());
                        stash.swap_remove(drop_idx);
                    }
                } else {
                    drop(b);
                }
            }
            // drop leftovers
            black_box(stash);
        });
    });
}
