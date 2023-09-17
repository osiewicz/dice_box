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
pub use unit_graph::UnitGraph;

pub fn create_dependency_queue(graph: unit_graph::UnitGraph) -> DependencyQueue {
    let hints = Box::new(runner::NoHintsProvider);
    let mut ret = DependencyQueue::new(hints);
    ret
}
