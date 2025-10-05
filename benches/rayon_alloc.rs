#[cfg(target_os = "hermit")]
use hermit as _;

use criterion::{criterion_group, criterion_main, Criterion};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use rayon::ThreadPoolBuilder;
use std::time::Instant;

#[cfg(feature = "virtual_alloc")]
use virtual_alloc::allocator::VirtualAlloc;

const KB: usize = 1 << 10;
pub const MB: usize = KB << 10;
pub const GB: usize = MB << 10;
pub const TB: usize = GB << 10;

#[cfg(feature = "virtual_alloc")]
#[global_allocator]
static ALLOC: VirtualAlloc<64> = VirtualAlloc::new(7 * GB, 16 * TB);

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Main", |b| b.iter(|| rayon_boxes()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

/// Parallel randomized alloc/free patterns using plain Box<Vec<u8>>.
fn rayon_boxes() {
    #[cfg(feature = "virtual_alloc")]
    {
        println!("waiting for init");
        ALLOC.spin_local_init().unwrap();
        println!("init");
    }

    let threads = std::thread::available_parallelism().map(|n| n.get() - 1).unwrap_or(1);
    let pool = ThreadPoolBuilder::new().num_threads(threads).build().unwrap();

    let tasks = threads * 02;         // number of parallel tasks
    let iters_per_task = 10_000;      // allocations per task
    let max_size = 16 * MB;           // max allocation size
    let keep_prob = 0.01f64;          // chance to keep an allocation across iterations
    let keep_max = 192;               // max allocations to keep

    let start = Instant::now();
    pool.install(|| {
        (0..tasks).into_par_iter().for_each(|t| {
            let mut rng = SmallRng::seed_from_u64(0xA110C ^ (t as u64));
            let mut stash: Vec<Box<[u8]>> = Vec::with_capacity(1024);

            for _ in 0..iters_per_task {
                // allocate a random size
                let sz = rng.random_range(1..=max_size);
                let mut b = vec![0u8; sz].into_boxed_slice();
                // write to memory
                if sz >= 8 {
                    let last = sz - 1;
                    b[0] = 1; b[last] = 2;
                }
                // randomly keep some allocations
                if rng.random_bool(keep_prob) {
                    stash.push(b);
                    // free older items
                    if stash.len() > keep_max {
                        let drop_idx = rng.random_range(0..stash.len());
                        stash.swap_remove(drop_idx);
                    }
                } else {
                    drop(b);
                }
            }
            // drop leftovers
            drop(stash);
        });
    });
    let dur = start.elapsed();
    let total_ops = (tasks as u64) * (iters_per_task as u64);
    println!(
        "[rayon-alloc (max:2^{},tasks:{},iters:{},keep:{}@{})] threads={} ops={} secs={:.3} ops/s={:.0} ({})",
        max_size.count_zeros(),
        tasks,
        iters_per_task,
        keep_max,
        keep_prob,
        threads,
        total_ops,
        dur.as_secs_f64(),
        (total_ops as f64) / dur.as_secs_f64(),
        if cfg!(feature = "virtual_alloc") { "virtual_alloc" } else { "native" },
    );

    #[cfg(feature = "virtual_alloc")]
    ALLOC.debug_dump_shards();
}

// import rayon parallel iter
use rayon::prelude::*;
