//! Parser for the timings file.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum BuildMode {
    RunCustomBuild,
    Build,
}
type PackageId = String;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct TimingInfo {
    mode: BuildMode,
    duration: f64,
    rmeta_time: Option<f64>,
    package_id: String,
    target: Target,
}

impl TimingInfo {
    fn node_type(&self) -> super::artifact::ArtifactType {
        match (self.mode, self.target.is_build_script()) {
            (BuildMode::Build, true) => crate::artifact::ArtifactType::BuildScriptBuild,
            (BuildMode::RunCustomBuild, true) => crate::artifact::ArtifactType::BuildScriptRun,
            (BuildMode::Build, false) => crate::artifact::ArtifactType::Metadata,
            (BuildMode::RunCustomBuild, false) => unreachable!(),
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
struct Target {
    name: String,
}

impl Target {
    fn is_build_script(&self) -> bool {
        self.name == "build-custom-build"
    }
}

pub(super) fn parse(contents: String) -> HashMap<PackageId, TimingInfo> {
    let mut out = HashMap::new();
    for line in contents.lines() {
        if !line.starts_with('{') {
            continue;
        }
        let timing: TimingInfo = serde_json::from_str(line).unwrap();
        out.insert(timing.package_id.clone(), timing).unwrap();
    }
    out
}
