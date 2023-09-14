use std::path::PathBuf;

use clap::Parser;

/// Dice_box - a testing ground for better Cargo scheduler.
#[derive(Parser)]
pub struct Cli {
    /// Input file
    pub timings_file: PathBuf,

    /// Output file
    pub dependency_graph_file: PathBuf,
}
