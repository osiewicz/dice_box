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

use crate::artifact::Artifact;

#[derive(Debug)]
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

    hints: Box<dyn super::runner::HintProvider>,
}

impl DependencyQueue {
    pub fn new(hints: Box<dyn super::runner::HintProvider>) -> Self {
        DependencyQueue {
            dep_map: BTreeMap::new(),
            reverse_dep_map: BTreeMap::new(),
            hints,
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
        let key = self
            .hints
            .suggest_next(&candidates)
            .or_else(|| candidates.into_iter().next())?
            .clone();
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
}
