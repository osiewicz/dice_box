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
        let reverse_dependencies = super::dependency_queue::reverse_dependencies(dependencies);
        let mut n_hints: Vec<Artifact> = vec![];
        for item in top_n_entries.into_iter() {
            let self_time = timings[&item].duration;
            let my_dependants = &reverse_dependencies[&item];
            let insertion_index: usize = (|| {
                if n_hints.is_empty() {
                    return 0;
                }
                let my_last_dependency = n_hints
                    .iter()
                    .rposition(|entry| reverse_dependencies[&entry].contains(&item));
                let my_first_dependant = n_hints
                    .iter()
                    .position(|entry| my_dependants.contains(entry));
                if let Some((my_last_dependency, my_first_dependant)) =
                    my_last_dependency.as_ref().zip(my_first_dependant.as_ref())
                {
                    assert!(
                        my_last_dependency < my_first_dependant,
                        "dependency: {}, dependant: {}, n_hints: {:?}",
                        my_last_dependency,
                        my_first_dependant,
                        &n_hints
                    );
                }

                n_hints
                    [my_last_dependency.unwrap_or(0)..my_first_dependant.unwrap_or(n_hints.len())]
                    .iter()
                    .position(|entry| {
                        let time = timings[&entry].duration;
                        time < self_time
                    })
                    .unwrap_or(my_first_dependant.unwrap_or(n_hints.len()))
            })();
            n_hints.insert(insertion_index, item);
        }
        let inner = CargoHints::new(dependencies, separate_codegen);
        dbg!(&n_hints);
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
                //assert_ne!(dependencies_of.len(), 0, "{:?}", artifact);

                self.n_hints
                    .iter()
                    .position(|a| &a == artifact || dependencies_of.contains(a))
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
                    .position(|a| &&a == f || dependencies_of.contains(a))
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
