//! Parser for the unit-graph file.
use serde::Deserialize;

/// 0-based index of Unit in `units` array of unit graph.
type UnitIndex = usize;

#[derive(Debug, Deserialize)]
struct Dependency {
    index: UnitIndex,
}

#[derive(Debug, Deserialize)]
struct Unit {
    pkg_id: String,
    target: super::timings::Target,
    mode: super::timings::BuildMode,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
pub struct UnitGraph {
    units: Vec<Unit>,
}
