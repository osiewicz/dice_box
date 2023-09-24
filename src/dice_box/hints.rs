use crate::artifact::{Artifact, ArtifactType};
pub(super) mod repeat_schedule;
/// Whenever Runner has a scheduling decision to make, it will consult it's hint provider.
pub trait HintProvider: std::fmt::Debug {
    fn suggest_next<'a>(&mut self, timings: &[&'a Artifact]) -> Option<&'a Artifact>;
}

/// Simple HintProvider that prefers units of particular type.
#[derive(Debug)]
pub(super) struct ChooseTypeProvider(ArtifactType);
impl HintProvider for ChooseTypeProvider {
    fn suggest_next<'a>(&mut self, timings: &[&'a Artifact]) -> Option<&'a Artifact> {
        timings.iter().find(|f| f.typ == self.0).cloned()
    }
}

impl ChooseTypeProvider {
    pub(super) fn new(typ: ArtifactType) -> Box<dyn HintProvider> {
        Box::new(Self(typ))
    }
}

/// AggregateHintProvider queries several underlying HintProvider's, returning the
/// query result of a first one that's successful.
#[derive(Debug)]
pub(super) struct AggregateHintProvider(Vec<Box<dyn HintProvider>>);

impl HintProvider for AggregateHintProvider {
    fn suggest_next<'a>(&mut self, timings: &[&'a Artifact]) -> Option<&'a Artifact> {
        self.0
            .iter_mut()
            .find_map(|provider| provider.suggest_next(timings))
    }
}

impl AggregateHintProvider {
    pub(super) fn new(hints: impl IntoIterator<Item = Box<dyn HintProvider>>) -> Box<Self> {
        Box::new(Self(hints.into_iter().collect()))
    }
}
