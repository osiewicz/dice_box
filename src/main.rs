use clap::Parser;
use dice_box::Cli;

fn main() {
    let opts = Cli::parse();

    println!("Dependency graph file: {:?}", opts.dependency_graph_file);

    println!("Timings file: {:?}", opts.timings_file);
}
