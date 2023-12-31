mod artifact;
mod cli;
mod dependency_queue;
mod hints;
mod runner;
mod timings;
mod unit_graph;

pub use cli::Cli;
pub use dependency_queue::CargoHints;
use dependency_queue::DependencyQueueBuilder;
pub use hints::NHintsProvider;
pub use runner::Runner;
pub use timings::parse;
pub use timings::Timings;
use unit_graph::unit_graph_to_artifacts;
pub use unit_graph::UnitGraph;
type PackageId = String;

pub fn create_dependency_queue(graph: unit_graph::UnitGraph) -> DependencyQueueBuilder {
    let mut ret = DependencyQueueBuilder::new();
    let artifact_units = unit_graph_to_artifacts(graph);
    for unit in artifact_units {
        ret.queue(unit.artifact, unit.dependencies);
    }
    ret
}
