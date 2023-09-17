mod artifact;
mod cli;
mod dependency_queue;
mod runner;
mod timings;
mod unit_graph;

use artifact::Artifact;
pub use cli::Cli;
use dependency_queue::DependencyQueue;
pub use runner::Runner;
pub use timings::parse;
use unit_graph::unit_graph_to_artifacts;
pub use unit_graph::UnitGraph;

pub fn create_dependency_queue(graph: unit_graph::UnitGraph) -> DependencyQueue {
    let hints = Box::new(runner::NoHintsProvider);
    let mut ret = DependencyQueue::new(hints);
    let artifact_units = unit_graph_to_artifacts(graph);
    for unit in artifact_units {
        ret.queue(unit.artifact, unit.dependencies);
    }
    ret
}
