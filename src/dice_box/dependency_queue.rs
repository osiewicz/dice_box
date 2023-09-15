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

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::artifact::Artifact;

#[derive(Debug)]
pub(super) struct DependencyQueue {
    /// A list of all known keys to build.
    ///
    /// The value of the hash map is list of dependencies which still need to be
    /// built before the package can be built. Note that the set is dynamically
    /// updated as more dependencies are built.
    dep_map: HashMap<Artifact, HashSet<Artifact>>,

    /// A reverse mapping of a package to all packages that depend on that
    /// package.
    ///
    /// This map is statically known and does not get updated throughout the
    /// lifecycle of the DependencyQueue.
    reverse_dep_map: HashMap<Artifact, HashSet<Artifact>>,

    /// The relative priority of this package. Higher values should be scheduled sooner.
    priority: HashMap<Artifact, usize>,

    /// An expected cost for building this package. Used to determine priority.
    cost: HashMap<Artifact, usize>,

    hints: Box<dyn super::runner::HintProvider>,
}

impl DependencyQueue {
    pub fn new(hints: Box<dyn super::runner::HintProvider>) -> Self {
        DependencyQueue {
            dep_map: HashMap::new(),
            reverse_dep_map: HashMap::new(),
            priority: HashMap::new(),
            cost: HashMap::new(),
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
    ///
    /// An optional `value` can also be associated with `key` which is reclaimed
    /// when the node is ready to go.
    ///
    /// The cost parameter can be used to hint at the relative cost of building
    /// this node. This implementation does not care about the units of this value, so
    /// the calling code is free to use whatever they'd like. In general, higher cost
    /// nodes are expected to take longer to build.
    pub fn queue(
        &mut self,
        key: Artifact,
        dependencies: impl IntoIterator<Item = Artifact>,
        cost: usize,
    ) {
        assert!(!self.dep_map.contains_key(&key));

        let mut my_dependencies = HashSet::new();
        for dep in dependencies {
            my_dependencies.insert(dep.clone());
            self.reverse_dep_map
                .entry(dep)
                .or_insert_with(HashSet::new)
                .insert(key.clone());
        }
        self.dep_map.insert(key.clone(), my_dependencies);
        self.cost.insert(key, cost);
    }

    /// All nodes have been added, calculate some internal metadata and prepare
    /// for `dequeue`.
    pub fn queue_finished(&mut self) {
        let mut out = HashMap::new();
        for key in self.dep_map.keys() {
            depth(key, &self.reverse_dep_map, &mut out);
        }
        self.priority = out
            .into_iter()
            .map(|(n, set)| {
                let total_cost =
                    self.cost[&n] + set.iter().map(|key| self.cost[key]).sum::<usize>();
                (n, total_cost)
            })
            .collect();

        /// Creates a flattened reverse dependency list. For a given key, finds the
        /// set of nodes which depend on it, including transitively. This is different
        /// from self.reverse_dep_map because self.reverse_dep_map only maps one level
        /// of reverse dependencies.
        fn depth<'a>(
            key: &Artifact,
            map: &HashMap<Artifact, HashSet<Artifact>>,
            results: &'a mut HashMap<Artifact, HashSet<Artifact>>,
        ) -> &'a HashSet<Artifact> {
            if results.contains_key(key) {
                let depth = &results[key];
                assert!(!depth.is_empty(), "cycle in DependencyQueue");
                return depth;
            }
            results.insert(key.clone(), HashSet::new());

            let mut set = HashSet::new();
            set.insert(key.clone());

            for dep in map.get(key).into_iter().flatten() {
                set.extend(depth(dep, map, results).iter().cloned())
            }

            let slot = results.get_mut(key).unwrap();
            *slot = set;
            &*slot
        }
    }

    /// Dequeues a package that is ready to be built.
    ///
    /// A package is ready to be built when it has 0 un-built dependencies. If
    /// `None` is returned then no packages are ready to be built.
    pub fn dequeue(&mut self) -> Option<Artifact> {
        let key = self
            .dep_map
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .next()?
            .0
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
