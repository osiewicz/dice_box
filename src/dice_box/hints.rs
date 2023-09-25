use crate::artifact::Artifact;
/// Whenever Runner has a scheduling decision to make, it will consult it's hint provider.
pub trait HintProvider: std::fmt::Debug {
    fn suggest_next<'a>(&mut self, timings: &[&'a Artifact]) -> Option<&'a Artifact>;
    fn label(&self) -> String;
}
