//! Parser for the unit-graph file.
use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::Deserialize;

use crate::{
    artifact::{Artifact, ArtifactType},
    timings::{node_type, BuildMode, CrateType, Target},
    PackageId,
};

/// 0-based index of Unit in `units` array of unit graph.
type UnitIndex = usize;

#[derive(Clone, Debug, Deserialize)]
/// Represents a Dependency within a particular unit graph.
pub(crate) struct Dependency {
    index: UnitIndex,
}

impl Dependency {
    pub(crate) fn new(index: UnitIndex) -> Self {
        Self { index }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Unit {
    pkg_id: PackageId,
    target: Target,
    mode: BuildMode,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ArtifactUnit {
    pub(crate) artifact: Artifact,
    pub(crate) dependencies: BTreeSet<Artifact>,
}

pub(crate) fn unit_graph_to_artifacts(graph: UnitGraph) -> Vec<ArtifactUnit> {
    fn unit_to_artifact(unit: &Unit) -> Artifact {
        let typ = node_type(&unit.mode, &unit.target);
        Artifact {
            typ,
            package_id: unit.pkg_id.clone(),
        }
    }
    let mut ret = BTreeMap::new();
    for unit in graph.units.iter() {
        let artifact = unit_to_artifact(&unit);
        let mut dependencies: Vec<_> = unit
            .dependencies
            .iter()
            .map(|dep| unit_to_artifact(&graph.units[dep.index]))
            .collect();
        assert!(!dependencies.contains(&artifact));
        if artifact.typ == ArtifactType::Metadata {
            ret.insert(
                Artifact {
                    typ: ArtifactType::Codegen,
                    package_id: artifact.package_id.clone(),
                },
                BTreeSet::from_iter([artifact.clone()]),
            );
        } else if artifact.typ == ArtifactType::Link
            || artifact.typ == ArtifactType::BuildScriptBuild
        {
            dependencies.iter_mut().for_each(|dep| {
                if dep.typ == ArtifactType::Metadata {
                    dep.typ = ArtifactType::Codegen;
                }
            });
        }
        ret.insert(artifact, BTreeSet::from_iter(dependencies));
    }
    fn depend_on_deps_of_deps(
        deps: &mut BTreeMap<Artifact, BTreeSet<Artifact>>,
        parent: &Artifact,
        child: &Artifact,
    ) {
        for dep in deps.get(child).cloned().unwrap() {
            if dep.typ == ArtifactType::Metadata {
                let should_recurse = deps.get_mut(parent).unwrap().insert(Artifact {
                    typ: ArtifactType::Codegen,
                    ..dep.clone()
                });
                if should_recurse {
                    depend_on_deps_of_deps(deps, &parent, &dep);
                }
            }
        }
    }
    let nodes: Vec<(Artifact, _)> = ret
        .iter()
        .filter(|(k, _)| k.typ == ArtifactType::Link)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (parent, deps) in nodes {
        for dep in deps {
            assert_ne!(dep, parent);
            depend_on_deps_of_deps(&mut ret, &parent, &dep);
        }
    }
    ret.into_iter()
        .map(|(artifact, dependencies)| ArtifactUnit {
            artifact,
            dependencies,
        })
        .collect()
}
#[derive(Clone, Debug, Deserialize)]
pub struct UnitGraph {
    pub(crate) units: Vec<Unit>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timings::CrateType;
    #[test]
    fn single_crate_build() {
        let unit_graph = UnitGraph {
            units: vec![
                Unit {
                    pkg_id: "target".into(),
                    target: Target {
                        name: "target".into(),
                        crate_types: vec![CrateType::Lib],
                    },
                    mode: BuildMode::Build,
                    dependencies: vec![],
                },
                Unit {
                    pkg_id: "target".into(),
                    target: Target {
                        name: "target".into(),
                        crate_types: vec![CrateType::Bin],
                    },
                    mode: BuildMode::Build,
                    dependencies: vec![Dependency::new(0)],
                },
            ],
        };
        let units = super::unit_graph_to_artifacts(unit_graph);
        let with_typ = |typ, deps: &[_]| ArtifactUnit {
            artifact: Artifact {
                typ,
                package_id: "target".into(),
            },
            dependencies: BTreeSet::from_iter(deps.iter().cloned().map(|typ| Artifact {
                typ,
                package_id: "target".into(),
            })),
        };
        assert_eq!(
            units,
            vec![
                with_typ(ArtifactType::Metadata, &[]),
                with_typ(ArtifactType::Codegen, &[ArtifactType::Metadata]),
                with_typ(ArtifactType::Link, &[ArtifactType::Codegen])
            ]
        );
    }
    #[test]
    fn single_crate_with_build_script() {
        let unit_graph = UnitGraph {
            units: vec![
                Unit {
                    pkg_id: "target".into(),
                    target: Target {
                        name: "build-script-build".into(),
                        crate_types: vec![CrateType::Lib],
                    },
                    mode: BuildMode::Build,
                    dependencies: vec![],
                },
                Unit {
                    pkg_id: "target".into(),
                    target: Target {
                        name: "build-script-build".into(),
                        crate_types: vec![CrateType::Bin],
                    },
                    mode: BuildMode::RunCustomBuild,
                    dependencies: vec![Dependency::new(0)],
                },
                Unit {
                    pkg_id: "target".into(),
                    target: Target {
                        name: "target".into(),
                        crate_types: vec![CrateType::Lib],
                    },
                    mode: BuildMode::Build,
                    dependencies: vec![Dependency::new(1)],
                },
                Unit {
                    pkg_id: "target".into(),
                    target: Target {
                        name: "target".into(),
                        crate_types: vec![CrateType::Bin],
                    },
                    mode: BuildMode::Build,
                    dependencies: vec![Dependency::new(2)],
                },
            ],
        };
        let units = super::unit_graph_to_artifacts(unit_graph);
        let with_typ = |typ, deps: &[_]| ArtifactUnit {
            artifact: Artifact {
                typ,
                package_id: "target".into(),
            },
            dependencies: BTreeSet::from_iter(deps.iter().cloned().map(|typ| Artifact {
                typ,
                package_id: "target".into(),
            })),
        };
        assert_eq!(
            units,
            vec![
                with_typ(ArtifactType::BuildScriptBuild, &[]),
                with_typ(
                    ArtifactType::BuildScriptRun,
                    &[ArtifactType::BuildScriptBuild]
                ),
                with_typ(ArtifactType::Metadata, &[ArtifactType::BuildScriptRun]),
                with_typ(ArtifactType::Codegen, &[ArtifactType::Metadata]),
                with_typ(ArtifactType::Link, &[ArtifactType::Codegen])
            ]
        );
    }
}
