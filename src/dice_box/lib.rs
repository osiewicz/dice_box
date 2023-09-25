mod artifact;
mod cli;
mod dependency_queue;
mod hints;
mod runner;
mod timings;
mod unit_graph;

pub use cli::Cli;
use dependency_queue::{DependencyQueue, DependencyQueueBuilder};
pub use runner::Runner;
pub use timings::parse;
use unit_graph::unit_graph_to_artifacts;
pub use unit_graph::UnitGraph;
type PackageId = String;

pub fn create_dependency_queue(
    graph: unit_graph::UnitGraph,
    separate_codegen: bool,
) -> DependencyQueue {
    let mut ret = DependencyQueueBuilder::new();
    let artifact_units = unit_graph_to_artifacts(graph);
    for unit in artifact_units {
        ret.queue(unit.artifact, unit.dependencies);
    }
    let hints = dependency_queue::CargoHints::new(&ret, separate_codegen);
    ret.finish(hints)
}
