use clap::Parser;
use dice_box::{Cli, Runner};
use tabled::Table;

fn main() {
    env_logger::init();
    let opts = Cli::parse();

    let timings_contents = std::fs::read_to_string(&opts.timings_file).unwrap();
    let timings = dice_box::parse(timings_contents);
    let unit_graph = std::fs::read_to_string(&opts.unit_graph_file).unwrap();
    let unit_graph: dice_box::UnitGraph = serde_json::from_str(&unit_graph).unwrap();
    let dependency_queue = dice_box::create_dependency_queue(unit_graph);
    let dep_graph_separate_codegen = {
        let hints = dice_box::CargoHints::new(&dependency_queue, true);
        dependency_queue.clone().finish(hints)
    };
    let mut dep_graph_n = {
        let hints = dice_box::NHintsProvider::new(&dependency_queue, &timings, false);
        dependency_queue.clone().finish(hints)
    };
    let dep_graph = {
        let hints = dice_box::CargoHints::new(&dependency_queue, false);
        dependency_queue.finish(hints)
    };

    let mut scenarios = [
        dice_box::Runner::new(dep_graph, timings.clone(), opts.num_threads),
        dice_box::Runner::new(
            dep_graph_separate_codegen,
            timings.clone(),
            opts.num_threads,
        ),
        dice_box::Runner::new(dep_graph_n, timings, opts.num_threads),
    ];
    let results = Table::new(scenarios.iter_mut().map(Runner::calculate)).to_string();
    println!("{}", results);
}
