use std::path::PathBuf;

use clap::Parser;

/// Dice_box - a testing ground for better Cargo scheduler.
#[derive(Parser)]
pub struct Cli {
    /// Timings file obtained with e.g. `cargo +nightly build --timings=json`
    pub timings_file: PathBuf,

    /// Unit graph file obtained with e.g. `cargo +nightly build --unit-graph`
    pub unit_graph_file: PathBuf,

    /// Number of threads in simulated build environment.
    #[clap(short, long, default_value_t = 10)]
    pub num_threads: usize,

    /// Whether to output timings for builds.
    #[clap(short, long)]
    pub timings: bool,
}
