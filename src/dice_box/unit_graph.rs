//! Parser for the unit-graph file.
use serde::Deserialize;

use crate::{artifact::Artifact, timings::node_type, PackageId};

/// 0-based index of Unit in `units` array of unit graph.
type UnitIndex = usize;

#[derive(Debug, Deserialize)]
pub(crate) struct Dependency {
    index: UnitIndex,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Unit {
    pkg_id: PackageId,
    target: super::timings::Target,
    mode: super::timings::BuildMode,
    dependencies: Vec<Dependency>,
}

pub(crate) struct ArtifactUnit {
    pub(crate) artifact: Artifact,
    pub(crate) dependencies: Vec<Artifact>,
}

pub(crate) fn unit_graph_to_artifacts(graph: UnitGraph) -> Vec<ArtifactUnit> {
    fn unit_to_artifact(unit: &Unit) -> Artifact {
        let typ = node_type(&unit.mode, &unit.target);
        Artifact {
            typ,
            package_id: unit.pkg_id.clone(),
        }
    }
    let mut ret = vec![];
    for unit in graph.units.iter() {
        let artifact = unit_to_artifact(&unit);
        let dependencies = unit
            .dependencies
            .iter()
            .map(|dep| unit_to_artifact(&graph.units[dep.index]))
            .collect();
        ret.push(ArtifactUnit {
            artifact,
            dependencies,
        });
    }
    ret
}
#[derive(Debug, Deserialize)]
pub struct UnitGraph {
    pub(crate) units: Vec<Unit>,
}
