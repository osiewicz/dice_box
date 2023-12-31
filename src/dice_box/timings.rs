//! Parser for the timings file.
mod visualization;
pub use visualization::{Timings, UnitTime};

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    artifact::{Artifact, ArtifactType},
    PackageId,
};
#[derive(Clone, Copy, Debug, Hash, Serialize, Deserialize, PartialEq, PartialOrd, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BuildMode {
    RunCustomBuild,
    Build,
}

// Parsed output of --timings=json
#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct TimingInfo {
    pub mode: BuildMode,
    pub duration: f64,
    pub rmeta_time: Option<f64>,
    pub package_id: PackageId,
    pub target: Target,
}

/// Input to cargo-timings-esque file generation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct TimingInstant {
    start: f64,
    #[serde(flatten)]
    info: TimingInfo,
}
pub(crate) fn node_type(mode: &BuildMode, target: &Target) -> ArtifactType {
    match (mode, target.is_build_script()) {
        (BuildMode::Build, true) => ArtifactType::BuildScriptBuild,
        (BuildMode::RunCustomBuild, true) => ArtifactType::BuildScriptRun,
        (BuildMode::Build, false)
            if target.crate_types.iter().any(|typ| {
                [
                    CrateType::Bin,
                    CrateType::ProcMacro,
                    CrateType::Rlib,
                    CrateType::Cdylib,
                ]
                .contains(typ)
            }) =>
        {
            ArtifactType::Link
        }
        (BuildMode::Build, false) => ArtifactType::Metadata,

        (BuildMode::RunCustomBuild, false) => unreachable!("{target:?}"),
    }
}
impl TimingInfo {
    fn node_type(&self) -> ArtifactType {
        node_type(&self.mode, &self.target)
    }
}
#[derive(Clone, Debug, Hash, Deserialize, Serialize, PartialEq, PartialOrd, Eq)]
#[serde(rename_all = "kebab-case")]
enum CrateType {
    Lib,
    ProcMacro,
    Rlib,
    Cdylib,
    Bin,
}
#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, PartialOrd, Eq)]
pub struct Target {
    pub name: String,
    crate_types: Vec<CrateType>,
}

impl Target {
    fn is_build_script(&self) -> bool {
        self.name == "build-script-build" || self.name == "build-script-main"
    }
}

/// Deserialize timings from contents of a timings.json file.
pub fn parse(contents: String) -> BTreeMap<Artifact, TimingInfo> {
    let mut out = BTreeMap::new();
    for line in contents.lines() {
        if !line.starts_with('{') {
            continue;
        }
        let mut timing: TimingInfo = serde_json::from_str(line).unwrap();
        let typ = timing.node_type();
        if typ == ArtifactType::Metadata {
            // Pipelining support
            assert!(
                timing.rmeta_time.is_some(),
                "{:?}",
                timing.target.crate_types
            );
            let mut codegen_timing = timing.clone();
            // Normalize codegen time
            codegen_timing.duration -= codegen_timing.rmeta_time.take().unwrap();
            out.insert(
                Artifact {
                    package_id: timing.package_id.clone(),
                    typ: ArtifactType::Codegen,
                },
                codegen_timing,
            );
            // ... and for Metadata unit we're about to insert, just use rmeta_time
            timing.duration = timing.rmeta_time.take().unwrap();
        }
        let _ = out.insert(
            Artifact {
                package_id: timing.package_id.clone(),
                typ,
            },
            timing,
        );
    }
    out
}
