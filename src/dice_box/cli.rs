use std::path::PathBuf;

use clap::Parser;

/// Dice_box - a testing ground for better Cargo scheduler.
#[derive(Parser)]
pub struct Cli {
    /// Input file
    pub timings_file: PathBuf,

    /// Output file
    pub dependency_graph_file: PathBuf,

    #[clap(short, long, default_value_t = 10)]
    pub num_threads: usize,

    /// Whether to treat codegen of a crate as a separate step from metadata generation.
    #[clap(long, default_value_t = false)]
    pub separate_codegen: bool,
}
