//! Parser for the timings file.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    artifact::{Artifact, ArtifactType},
    PackageId,
};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum BuildMode {
    RunCustomBuild,
    Build,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct TimingInfo {
    mode: BuildMode,
    pub duration: f64,
    rmeta_time: Option<f64>,
    package_id: PackageId,
    target: Target,
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
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
pub(super) enum CrateType {
    Lib,
    ProcMacro,
    Rlib,
    Cdylib,
    Bin,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub(crate) struct Target {
    pub(super) name: String,
    pub(super) crate_types: Vec<CrateType>,
}

impl Target {
    fn is_build_script(&self) -> bool {
        self.name == "build-script-build" || self.name == "build-script-main"
    }
}

pub fn parse(contents: String) -> BTreeMap<Artifact, TimingInfo> {
    let timings: Vec<TimingInfo> = serde_json::from_str(&contents).unwrap();
    let mut out = BTreeMap::new();
    for mut timing in timings {
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
