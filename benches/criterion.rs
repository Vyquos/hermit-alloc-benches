#[cfg(target_os = "hermit")]
use hermit as _;

use alloc_benches::{human_bytes, rayon_boxes, Params, ALLOC};
#[allow(unused_imports)]
use alloc_benches::{KB, MB, GB, TB};

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
    PlottingBackend, Throughput,
};
use rayon::ThreadPoolBuilder;
use std::path::Path;
use std::time::Duration;

fn criterion() -> Criterion {
    Criterion::default()
        .configure_from_args()
        .plotting_backend(PlottingBackend::Plotters)
        .output_directory(Path::new("/root/benches"))
}

criterion_group!(name = benches; config = criterion(); targets = bench_allocators);
criterion_main!(benches);

fn bench_allocators(c: &mut Criterion) {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let keep_maxes = [0, 64, 256];
    let max_sizes  = [64*KB, 1*MB];
    let iters      = [10_000, 20_000];

    for &max_size in &max_sizes {
        let mut group = c.benchmark_group(format!(
            "rayon_boxes/max_size={}/{}",
            human_bytes(max_size),
            if cfg!(feature = "virtual_alloc") {
                "virtual_alloc"
            } else {
                "native"
            }
        ));
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Linear));
        group.measurement_time(Duration::from_secs(30));
        group.sample_size(20);

        for &iters_per_task in &iters {
            for &keep_max in &keep_maxes {
                let p = Params {
                    keep_max,
                    keep_prob: 0.01,
                    max_size,
                    iters_per_task,
                    sample_size: 0,
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
                    ALLOC.enable();
                    b.iter(|| rayon_boxes(&pool, params));
                });
            }
        }
        group.finish();

        ALLOC.disable();
        unsafe { ALLOC.reset_shards() };
    }
}
