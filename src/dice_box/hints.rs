use std::collections::{BTreeMap, BTreeSet};

use crate::{
    artifact::{Artifact, ArtifactType},
    dependency_queue::DependencyQueueBuilder,
    timings::TimingInfo,
    CargoHints,
};
/// Whenever Runner has a scheduling decision to make, it will consult it's hint provider.
pub trait HintProvider: std::fmt::Debug {
    fn suggest_next<'a>(&mut self, timings: &[&'a Artifact]) -> Option<&'a Artifact>;
    fn label(&self) -> String;
}

#[derive(Debug)]
pub struct NHintsProvider {
    n_hints: Vec<Artifact>,
    inner: Box<dyn HintProvider>,
    reverse_dependencies: BTreeMap<Artifact, BTreeSet<Artifact>>,
    separate_codegen: bool,
}

impl NHintsProvider {
    pub fn new(
        dependencies: &DependencyQueueBuilder,
        timings: &BTreeMap<Artifact, TimingInfo>,
        separate_codegen: bool,
    ) -> Box<dyn HintProvider> {
        let mut top_n_entries = timings.iter().map(|(a, b)| (b, a)).collect::<Vec<_>>();
        top_n_entries.sort_by_key(|entry| ordered_float::OrderedFloat(entry.0.duration));
        let top_n_entries: Vec<Artifact> = top_n_entries
            .into_iter()
            .map(|(_, artifact)| artifact.clone())
            .rev()
            .take(100)
            .collect();
        let mut reverse_dependencies = BTreeMap::new();
        for key in dependencies.dep_map.keys() {
            super::dependency_queue::depth(
                key,
                &dependencies.reverse_dep_map,
                &mut reverse_dependencies,
            );
        }
        let mut n_hints: Vec<Artifact> = vec![];
        for item in top_n_entries.into_iter() {
            let self_time = timings[&item].duration;
            let my_dependants = &reverse_dependencies[&item];
            let insertion_index: usize = (|| {
                for (index, entry) in n_hints.iter().enumerate() {
                    if my_dependants.contains(&entry) {
                        // We've encountered our dependency, so we must push it to the right
                        // and take it's slot.
                        return index;
                    }
                    if reverse_dependencies[&entry].contains(&item) {
                        // Something that we depend on is okay, we just must ensure that we're
                        // built after it.
                        continue;
                    }
                    let time = timings[&entry].duration;
                    if time < self_time {
                        if index != n_hints.len() - 1 {
                            assert!(n_hints[index + 1..]
                                .iter()
                                .all(|entry| !my_dependants.contains(entry)));
                        }
                        return index;
                    }
                }
                return n_hints.len();
            })();
            n_hints.insert(insertion_index, item);
        }
        let inner = CargoHints::new(dependencies, separate_codegen);
        Box::new(Self {
            n_hints,
            inner,
            reverse_dependencies,
            separate_codegen,
        })
    }
}
impl HintProvider for NHintsProvider {
    fn suggest_next<'a>(&mut self, timings: &[&'a Artifact]) -> Option<&'a Artifact> {
        if !self.separate_codegen {
            if let Some(t) = timings
                .iter()
                .find(|item| item.typ == ArtifactType::Codegen)
            {
                return Some(t);
            }
        }
        let Some(min_position) = timings
            .iter()
            .filter_map(|artifact| {
                let dependencies_of = &self.reverse_dependencies[&artifact];
                assert_ne!(dependencies_of.len(), 0);

                self.n_hints
                    .iter()
                    .position(|a| dependencies_of.contains(a))
            })
            .min()
        else {
            return self.inner.suggest_next(&timings);
        };
        let candidates = timings
            .iter()
            .filter(|f| {
                let dependencies_of = &self.reverse_dependencies[&f];
                self.n_hints
                    .iter()
                    .position(|a| dependencies_of.contains(a))
                    == Some(min_position)
            })
            .cloned()
            .collect::<Vec<_>>();
        self.inner.suggest_next(&candidates)
    }

    fn label(&self) -> String {
        "N-Hints".into()
    }
}
