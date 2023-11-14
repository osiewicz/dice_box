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
    let dep_graph_n = {
        let hints = dice_box::NHintsProvider::new(&dependency_queue, &timings);
        dependency_queue.clone().finish(hints)
    };
    let dep_graph = {
        let hints = dice_box::CargoHints::new(&dependency_queue);
        dependency_queue.clone().finish(hints)
    };
    let optimal_dep_graph = {
        let hints = dice_box::CargoHints::new(&dependency_queue);
        dependency_queue.finish(hints)
    };
    let mut scenarios = [
        dice_box::Runner::new(dep_graph, timings.clone(), opts.num_threads),
        dice_box::Runner::new(dep_graph_n, timings.clone(), opts.num_threads),
        dice_box::Runner::new(optimal_dep_graph, timings, u8::MAX as usize)
            .with_label("Optimal build schedule (current Cargo algo)".into()),
    ];
    let (results, timings): (Vec<_>, Vec<_>) = scenarios
        .iter_mut()
        .map(|runner| runner.calculate())
        .unzip();
    let results = Table::new(results).to_string();
    println!("{}", results);
    timings.into_iter().enumerate().for_each(|(index, timing)| {
        // timing.report_html(index.to_string()).ok();
    });
}
