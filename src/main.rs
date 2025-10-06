#[cfg(target_os = "hermit")]
use hermit as _;

use alloc_benches::{rayon_boxes, Params};
#[allow(unused_imports)]
use alloc_benches::{KB, MB, GB, TB};

use rayon::ThreadPoolBuilder;
use serde::Serialize;
use std::fs::{create_dir_all, File};
use std::io::{self, BufWriter};
use std::time::Instant;

#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkData {
    params: Params,
    timings: Vec<f64>,
}

const OUTPUT_PATH: &str = "/root/benches/manual-benches";

fn main() {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let params = Params {
        keep_max: 128,
        keep_prob: 0.01,
        max_size: 1 * MB,
        iters_per_task: 10_000,
        sample_size: 20,
        tasks: 2 * threads,
        threads,
    };

    let writer = setup_stats(params).unwrap();

    let pool = ThreadPoolBuilder::new()
        .num_threads(params.threads)
        .build()
        .unwrap();

    let data = run_bench(&pool, params);

    write_stats(writer, data).unwrap();
    println!("done");
}

pub fn setup_stats(params: Params) -> io::Result<BufWriter<File>> {
    let dirname = format!(
        "{OUTPUT_PATH}/{}_x{}",
        if cfg!(feature = "virtual_alloc") {
            "virtual_alloc"
        } else {
            "native"
        },
        params.threads,
    );
    create_dir_all(&dirname)?;
    let filename = format!(
        "{dirname}/rayon-bench_iter{}_max{}_keep{}.json",
        params.iters_per_task * params.tasks,
        params.max_size,
        params.keep_max
    );
    println!("creating {filename}");
    let file = File::create(filename)?;
    Ok(BufWriter::new(file))
}

pub fn write_stats<W: io::Write>(
    writer: BufWriter<W>,
    data: BenchmarkData,
) -> serde_json::Result<()> {
    serde_json::to_writer_pretty(writer, &data)?;
    Ok(())
}

pub fn run_bench(pool: &rayon::ThreadPool, params: Params) -> BenchmarkData {
    let mut data = BenchmarkData {
        params: params,
        timings: Vec::with_capacity(params.sample_size),
    };
    for _ in 0..params.sample_size {
        let start = Instant::now();
        rayon_boxes(&pool, params);
        data.timings.push(start.elapsed().as_secs_f64());
    }
    data
}
