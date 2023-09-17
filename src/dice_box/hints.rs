use crate::artifact::{Artifact, ArtifactType};

/// Whenever Runner has a scheduling decision to make, it will consult it's hint provider.
pub trait HintProvider: std::fmt::Debug {
    fn suggest_next<'a>(&self, timings: &[&'a Artifact]) -> Option<&'a Artifact> {
        timings
            .iter()
            .find(|f| f.typ == ArtifactType::Metadata)
            .cloned()
    }
}

#[derive(Debug)]
pub(super) struct NoHintsProvider;
impl HintProvider for NoHintsProvider {}
