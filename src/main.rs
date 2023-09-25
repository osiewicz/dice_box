use clap::Parser;
use dice_box::Cli;

fn main() {
    let opts = Cli::parse();

    let timings_contents = std::fs::read_to_string(&opts.timings_file).unwrap();
    let timings = dice_box::parse(timings_contents);
    println!();
    let unit_graph = std::fs::read_to_string(&opts.unit_graph_file).unwrap();
    let unit_graph: dice_box::UnitGraph = serde_json::from_str(&unit_graph).unwrap();
    let dep_graph = dice_box::create_dependency_queue(unit_graph, opts.separate_codegen);
    let mut runner = dice_box::Runner::new(dep_graph, timings, opts.num_threads);
    println!("{:?}", runner.calculate());
}
