#[cfg(target_os = "hermit")]
use hermit as _;

use alloc_benches::{human_bytes, rayon_boxes, Params};
#[allow(unused_imports)]
use alloc_benches::{KB, MB, GB, TB};

use clap::Parser;
use rayon::ThreadPoolBuilder;
use serde::Serialize;
use std::fs::{create_dir_all, File};
use std::io::{self, BufWriter};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Parser)]
pub struct Options {
    #[arg(short, long)]
    pub keep_max: Option<usize>,
    #[arg(long)]
    pub keep_prob: Option<f64>,
    #[arg(short, long)]
    pub max_size: Option<usize>,
    #[arg(short, long)]
    pub iters_per_task: Option<usize>,
    #[arg(short, long)]
    pub sample_size: Option<usize>,
    #[arg(short, long)]
    pub tasks: Option<usize>,
}

impl From<Options> for Params {
    fn from(opt: Options) -> Self {
        let defaults = Params::default();
        Params {
            keep_max: opt.keep_max.unwrap_or(defaults.keep_max),
            keep_prob: opt.keep_prob.unwrap_or(defaults.keep_prob),
            max_size: opt.max_size.unwrap_or(defaults.max_size),
            iters_per_task: opt.iters_per_task.unwrap_or(defaults.iters_per_task),
            sample_size: opt.sample_size.unwrap_or(defaults.sample_size),
            tasks: opt.tasks.unwrap_or(defaults.tasks),
            threads: defaults.threads,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkData {
    params: Params,
    timings: Vec<f64>,
}

const OUTPUT_PATH: &str = "/root/benches/manual-benches";

fn main() {
    let params = Options::parse().into();

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
        human_bytes(params.max_size),
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
