//! This module was pulled out of Cargo (4b84887848a31c6f83434cee2135f4fb0e2c9cf3).
//!
//! A graph-like structure used to represent a set of dependencies and in what
//! order they should be built.
//!
//! This structure is used to store the dependency graph and dynamically update
//! it to figure out when a dependency should be built.
//!
//! Dependencies in this queue are represented as a (node, edge) pair. This is
//! used to model nodes which produce multiple outputs at different times but
//! some nodes may only require one of the outputs and can start before the
//! whole node is finished.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    artifact::{Artifact, ArtifactType},
    hints::HintProvider,
};

#[derive(Clone, Debug)]
pub struct DependencyQueueBuilder {
    /// A list of all known keys to build.
    ///
    /// The value of the hash map is list of dependencies which still need to be
    /// built before the package can be built. Note that the set is dynamically
    /// updated as more dependencies are built.
    pub(super) dep_map: BTreeMap<Artifact, BTreeSet<Artifact>>,

    /// A reverse mapping of a package to all packages that depend on that
    /// package.
    ///
    /// This map is statically known and does not get updated throughout the
    /// lifecycle of the DependencyQueue.
    pub(super) reverse_dep_map: BTreeMap<Artifact, BTreeSet<Artifact>>,
}

/// Analog of Cargo's DependencyQueue except of
/// - being non-generic to make it a bit easier to experiment with - and not battle the trait bounds.
/// - Excluding Job type - as we never actually execute builds.
/// - Excluding `priority` and `cost` members, which are available as a HintProvider implementation in [CargoHints].
/// This type also relies on this crate's HintProvider which makes scheduling decisions.
/// Oh, an there's a [DependencyQueueBuilder] for it too, and for a reason; \
/// some HintProviders might want to inspect the finished queue during it's initialization, which leads to circular dependency between
/// a DependencyQueue and HintProvider implementation.
/// We also use a BTreeMap instead of HashMap in this DependencyQueue to make the results of makespan simulation fully
/// deterministic - we must not depend on the order of iteration here.
pub struct DependencyQueue {
    /// A list of all known keys to build.
    ///
    /// The value of the hash map is list of dependencies which still need to be
    /// built before the package can be built. Note that the set is dynamically
    /// updated as more dependencies are built.
    dep_map: BTreeMap<Artifact, BTreeSet<Artifact>>,

    /// A reverse mapping of a package to all packages that depend on that
    /// package.
    ///
    /// This map is statically known and does not get updated throughout the
    /// lifecycle of the DependencyQueue.
    reverse_dep_map: BTreeMap<Artifact, BTreeSet<Artifact>>,
    hints: Box<dyn super::hints::HintProvider>,
}

impl DependencyQueueBuilder {
    pub fn new() -> Self {
        Self {
            dep_map: BTreeMap::new(),
            reverse_dep_map: BTreeMap::new(),
        }
    }
    /// Adds a new node and its dependencies to this queue.
    ///
    /// The `key` specified is a new node in the dependency graph, and the node
    /// depend on all the dependencies iterated by `dependencies`. Each
    /// dependency is a node/edge pair, where edges can be thought of as
    /// productions from nodes (aka if it's just `()` it's just waiting for the
    /// node to finish).
    pub fn queue(&mut self, key: Artifact, dependencies: impl IntoIterator<Item = Artifact>) {
        if self.dep_map.contains_key(&key) {
            return;
        }

        let mut my_dependencies = BTreeSet::new();
        for dep in dependencies {
            my_dependencies.insert(dep.clone());
            self.reverse_dep_map
                .entry(dep)
                .or_insert_with(BTreeSet::new)
                .insert(key.clone());
        }
        self.dep_map.insert(key.clone(), my_dependencies);
    }

    pub fn finish(self, hints: Box<dyn HintProvider>) -> DependencyQueue {
        DependencyQueue {
            dep_map: self.dep_map,
            reverse_dep_map: self.reverse_dep_map,
            hints,
        }
    }
}

impl DependencyQueue {
    /// Dequeues a package that is ready to be built.
    ///
    /// A package is ready to be built when it has 0 un-built dependencies. If
    /// `None` is returned then no packages are ready to be built.
    pub fn dequeue(&mut self) -> Option<Artifact> {
        let candidates: Vec<&Artifact> = self
            .dep_map
            .iter()
            .filter_map(|(artifact, deps)| deps.is_empty().then_some(artifact))
            .collect();
        let key = self.hints.suggest_next(&candidates)?.clone();
        let _ = self.dep_map.remove(&key).unwrap();
        Some(key)
    }

    /// Returns `true` if there are remaining packages to be built.
    pub fn is_empty(&self) -> bool {
        self.dep_map.is_empty()
    }

    /// Returns the number of remaining packages to be built.
    pub fn len(&self) -> usize {
        self.dep_map.len()
    }

    /// Indicate that something has finished.
    ///
    /// Calling this function indicates that the `node` has produced `edge`. All
    /// remaining work items which only depend on this node/edge pair are now
    /// candidates to start their job.
    ///
    /// Returns the nodes that are now allowed to be dequeued as a result of
    /// finishing this node.
    pub fn finish(&mut self, node: &Artifact) -> Vec<&Artifact> {
        // hashset<Artifactode>
        let reverse_deps = self.reverse_dep_map.get(node);
        let Some(reverse_deps) = reverse_deps else {
            return Vec::new();
        };
        let key = node.clone();
        let mut result = Vec::new();
        for dep in reverse_deps.iter() {
            let edges = &mut self.dep_map.get_mut(dep).unwrap();
            assert!(edges.remove(&key));
            if edges.is_empty() {
                result.push(dep);
            }
        }
        result
    }

    pub fn hints(&self) -> &dyn HintProvider {
        &*self.hints
    }
}

/// Scheduling implementation of Cargo as of 24.09.2023. It schedules dependencies based on potential parallelism
/// once that crate is built (which corresponds directly with # of it's dependants).
#[derive(Debug)]
pub struct CargoHints {
    priority: BTreeMap<Artifact, usize>,
    separate_codegen: bool,
}

impl HintProvider for CargoHints {
    fn suggest_next<'a>(&mut self, timings: &[&'a Artifact]) -> Option<&'a Artifact> {
        timings
            .iter()
            .max_by_key(|artifact| {
                (
                    // This is a customization point for separate-codegen implementation.
                    // As is today, Codegen and Metadata are bundled into once rustc process,
                    // which we emulate in scheduler by always scheduling codegen units straight away.
                    // If separate codegen is enabled, we allow Codegen to be scheduled later on.
                    self.separate_codegen || artifact.typ == ArtifactType::Codegen,
                    self.priority[artifact],
                )
            })
            .cloned()
    }
    fn label(&self) -> String {
        let suffix = if self.separate_codegen {
            " (separate codegen units)"
        } else {
            ""
        };
        format!("Cargo Hints{}", suffix)
    }
}

/// Creates a flattened reverse dependency list. For a given key, finds the
/// set of nodes which depend on it, including transitively. This is different
/// from self.reverse_dep_map because self.reverse_dep_map only maps one level
/// of reverse dependencies.
fn depth<'a>(
    key: &Artifact,
    map: &BTreeMap<Artifact, BTreeSet<Artifact>>,
    results: &'a mut BTreeMap<Artifact, BTreeSet<Artifact>>,
) -> &'a BTreeSet<Artifact> {
    if results.contains_key(key) {
        let depth = &results[key];
        assert!(!depth.is_empty(), "cycle in DependencyQueue");
        return depth;
    }
    results.insert(key.clone(), BTreeSet::new());

    let mut set = BTreeSet::new();
    set.insert(key.clone());

    for dep in map.get(key).into_iter().flat_map(|it| it.iter()) {
        set.extend(depth(dep, map, results).iter().cloned())
    }

    let slot = results.get_mut(key).unwrap();
    *slot = set;
    &*slot
}

pub(super) fn reverse_dependencies(
    deps: &DependencyQueueBuilder,
) -> BTreeMap<Artifact, BTreeSet<Artifact>> {
    let mut out = BTreeMap::new();
    for key in deps.dep_map.keys() {
        depth(key, &deps.reverse_dep_map, &mut out);
    }
    out
}

impl CargoHints {
    pub fn new(deps: &DependencyQueueBuilder, separate_codegen: bool) -> Box<dyn HintProvider> {
        let out = reverse_dependencies(&deps);
        fn dependent_cost(typ: ArtifactType) -> usize {
            if typ == ArtifactType::Codegen {
                0
            } else {
                10
            }
        }
        let priority = out
            .into_iter()
            .map(|(n, set)| {
                let total_cost = dependent_cost(n.typ)
                    + set
                        .iter()
                        .map(|dependent| dependent_cost(dependent.typ))
                        .sum::<usize>();
                (n, total_cost)
            })
            .collect();

        Box::new(Self {
            priority,
            separate_codegen,
        })
    }
}
