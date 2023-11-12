//! Parser for the unit-graph file.
use std::collections::HashSet;

use serde::Deserialize;

use crate::{
    artifact::{Artifact, ArtifactType},
    timings::node_type,
    PackageId,
};

/// 0-based index of Unit in `units` array of unit graph.
type UnitIndex = usize;

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Eq)]
pub(crate) struct Dependency {
    index: UnitIndex,
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Eq)]
pub(crate) struct Unit {
    pub(crate) pkg_id: PackageId,
    pub(crate) target: super::timings::Target,
    pub(crate) mode: super::timings::BuildMode,
    pub(crate) dependencies: Vec<Dependency>,
}

pub(crate) struct ArtifactUnit {
    pub(crate) artifact: Artifact,
    pub(crate) dependencies: HashSet<Artifact>,
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
        let mut dependencies: Vec<_> = unit
            .dependencies
            .iter()
            .map(|dep| unit_to_artifact(&graph.units[dep.index]))
            .collect();
        if artifact.typ == ArtifactType::Metadata {
            ret.push(ArtifactUnit {
                artifact: Artifact {
                    typ: ArtifactType::Codegen,
                    package_id: artifact.package_id.clone(),
                },
                dependencies: HashSet::from_iter([artifact.clone()]),
            });
        } else if artifact.typ == ArtifactType::Link
            || artifact.typ == ArtifactType::BuildScriptBuild
        {
            dependencies.iter_mut().for_each(|dep| {
                if dep.typ == ArtifactType::Metadata {
                    dep.typ = ArtifactType::Codegen;
                }
            })
        }

        ret.push(ArtifactUnit {
            artifact,
            dependencies: HashSet::from_iter(dependencies),
        });
    }
    ret
}
#[derive(Clone, Debug, Deserialize)]
pub struct UnitGraph {
    pub(crate) units: Vec<Unit>,
}
