#[cfg(target_os = "hermit")]
use hermit as _;

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
    PlottingBackend, Throughput,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::hint::black_box;
use std::path::Path;
use std::time::Duration;

#[cfg(feature = "virtual_alloc")]
use virtual_alloc::allocator::VirtualAlloc;

#[derive(Clone, Copy, Debug)]
struct Params {
    keep_max: usize,
    keep_prob: f64,
    max_size: usize,
    iters_per_task: usize,
    tasks: usize,
    threads: usize,
}

const KB: usize = 1 << 10;
pub const MB: usize = KB << 10;
pub const GB: usize = MB << 10;
pub const TB: usize = GB << 10;

#[cfg(feature = "virtual_alloc")]
#[global_allocator]
static ALLOC: VirtualAlloc<64> = VirtualAlloc::new(7 * GB, 16 * TB);

fn criterion() -> Criterion {
    Criterion::default()
        .configure_from_args()
        .plotting_backend(PlottingBackend::Plotters)
        .output_directory(Path::new("/root/benches"))
}

criterion_group!(name = benches; config = criterion(); targets = bench_allocators);
criterion_main!(benches);

/// Parallel randomized alloc/free patterns using plain Box<Vec<u8>>.
fn rayon_boxes(pool: &rayon::ThreadPool, params: Params) {
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

fn human_bytes(n: usize) -> String {
    if n >= MB {
        format!("{}MiB", n / MB)
    } else if n >= KB {
        format!("{}KiB", n / KB)
    } else {
        format!("{}B", n)
    }
}

fn bench_allocators(c: &mut Criterion) {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let keep_maxes = [0, 1, 2, 4, 8, 16, 32, 64, 128, 192, 256];
    let max_sizes  = [64*KB, 1*MB];
    let iters      = [10_000, 50_000];

    for &max_size in &max_sizes {
        let mut group =
            c.benchmark_group(format!("rayon_boxes/max_size={}", human_bytes(max_size)));
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Linear));
        group.measurement_time(Duration::from_secs(10));
        group.sample_size(30);

        for &iters_per_task in &iters {
            for &keep_max in &keep_maxes {
                let p = Params {
                    keep_max,
                    keep_prob: 0.01,
                    max_size,
                    iters_per_task,
                    tasks: 2 * threads,
                    threads,
                };

                // Per-iteration throughput metadata
                let total_allocs_per_iter = (p.tasks * p.iters_per_task) as u64;
                let bytes_per_iter = total_allocs_per_iter * 2;

                group.throughput(Throughput::Elements(total_allocs_per_iter));
                group.throughput(Throughput::Bytes(bytes_per_iter));

                let id = BenchmarkId::from_parameter(format!(
                    "steady,keep_max={keep_max},iters={iters_per_task}"
                ));
                group.bench_with_input(id, &p, |b, &params| {
                    let pool = ThreadPoolBuilder::new()
                        .num_threads(params.threads)
                        .build()
                        .unwrap();
                    b.iter(|| rayon_boxes(&pool, params));
                });
            }
        }
        group.finish();
    }
}
