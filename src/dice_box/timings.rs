//! Parser for the timings file.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::artifact::ArtifactType;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum BuildMode {
    RunCustomBuild,
    Build,
}
type PackageId = String;

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct TimingInfo {
    mode: BuildMode,
    duration: f64,
    rmeta_time: Option<f64>,
    package_id: String,
    target: Target,
}

impl TimingInfo {
    fn node_type(&self) -> ArtifactType {
        match (self.mode, self.target.is_build_script()) {
            (BuildMode::Build, true) => ArtifactType::BuildScriptBuild,
            (BuildMode::RunCustomBuild, true) => ArtifactType::BuildScriptRun,
            (BuildMode::Build, false) if self.rmeta_time.is_none() => ArtifactType::Link,
            (BuildMode::Build, false) => ArtifactType::Metadata,

            (BuildMode::RunCustomBuild, false) => unreachable!(),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
enum CrateType {
    Lib,
    ProcMacro,
    Bin,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub(crate) struct Target {
    name: String,
    crate_types: Vec<CrateType>,
}

impl Target {
    fn is_build_script(&self) -> bool {
        self.name == "build-script-build"
    }
}

pub fn parse(contents: String) -> BTreeMap<(PackageId, ArtifactType), TimingInfo> {
    let mut out = BTreeMap::new();
    for line in contents.lines() {
        if !line.starts_with('{') {
            continue;
        }
        let mut timing: TimingInfo = serde_json::from_str(line).unwrap();
        let kind = timing.node_type();
        if kind == ArtifactType::Metadata {
            assert!(timing.rmeta_time.is_some());
            let mut codegen_timing = timing.clone();
            // Normalize codegen time
            codegen_timing.duration -= codegen_timing.rmeta_time.take().unwrap();
            out.insert(
                (timing.package_id.clone(), ArtifactType::Codegen),
                codegen_timing,
            );
            // ... and for Metadata unit we're about to insert, just use rmeta_time
            timing.duration = timing.rmeta_time.take().unwrap();
        }
        let is_unique = out
            .insert((timing.package_id.clone(), kind), timing)
            .is_none();
        assert!(is_unique, "{line}");
    }
    out
}
