//! Timing visualization
//!
//! This module implements visualization of simulated build process. Large parts of it are pulled verbatim from cargo. Notably I've stripped tracking of units unlocked by finished rmeta/codegen.
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{BufWriter, Write};
use std::time::SystemTime;

use crate::artifact::{Artifact, ArtifactType};
use crate::runner::StartTime;
use crate::timings::BuildMode;
use crate::unit_graph::Unit;

/// Tracking information for the entire build.
///
/// Methods on this structure are generally called from the main thread of a
/// running [`JobQueue`] instance (`DrainState` in specific) when the queue
/// receives messages from spawned off threads.
///
/// [`JobQueue`]: super::JobQueue
pub struct Timings {
    /// A rendered string of when compilation started.
    start_str: String,
    /// Time tracking for each individual unit.
    unit_times: Vec<UnitTime>,
    /// Concurrency-tracking information. This is periodically updated while
    /// compilation progresses.
    concurrency: Vec<Concurrency>,
    /// Recorded CPU states, stored as tuples. First element is when the
    /// recording was taken and second element is percentage usage of the
    /// system.
    cpu_usage: Vec<(f64, f64)>,
    total_time: f64,
}

/// Tracking information for an individual unit.
pub struct UnitTime {
    unit: Unit,
    /// A string describing the cargo target.
    target: String,
    /// The time when this unit started as an offset in seconds from `Timings::start`.
    start: f64,
    /// Total time to build this unit in seconds.
    duration: f64,
    /// The time when the `.rmeta` file was generated, an offset in seconds
    /// from `start`.
    rmeta_time: Option<f64>,
}

/// Periodic concurrency tracking information.
#[derive(serde::Serialize)]
struct Concurrency {
    /// Time as an offset in seconds from `Timings::start`.
    t: f64,
    /// Number of units currently running.
    active: usize,
    /// Number of units that could run, but are waiting for a jobserver token.
    waiting: usize,
    /// Number of units that are not yet ready, because they are waiting for
    /// dependencies to finish.
    inactive: usize,
}

impl Timings {
    pub fn new(
        order: &[(StartTime, Artifact)],
        timings: &BTreeMap<Artifact, super::TimingInfo>,
        cores: usize,
        total_time: u64,
    ) -> Timings {
        let start_str = humantime::format_rfc3339_seconds(SystemTime::now()).to_string();
        let total_time = total_time as f64 / 1000.;
        type StartedUnits = usize;
        type EndedUnits = usize;
        let mut unique_times = BTreeMap::<u64, (StartedUnits, EndedUnits)>::new();
        for (start_time, item) in order.iter() {
            unique_times.entry(*start_time).or_default().0 += 1;
            let end_time = start_time + (timings.get(item).unwrap().duration * 1000.) as u64;
            unique_times.entry(end_time).or_default().1 += 1;
        }
        let mut unit_times: Vec<UnitTime> = vec![];
        for (start_time, item) in order.into_iter() {
            let info = timings.get(item).unwrap();
            let codegen_info = (item.typ == ArtifactType::Metadata)
                .then(|| {
                    timings.get(&Artifact {
                        typ: crate::artifact::ArtifactType::Codegen,
                        ..item.clone()
                    })
                })
                .flatten();
            let rmeta_time = codegen_info
                .map(|_| info.duration)
                .or_else(|| info.rmeta_time);
            let duration = codegen_info
                .map(|codegen| codegen.duration)
                .unwrap_or_default()
                + info.duration;
            unit_times.push(UnitTime {
                unit: Unit {
                    pkg_id: info.package_id.clone(),
                    target: info.target.clone(),
                    mode: info.mode.clone(),
                    dependencies: vec![],
                },
                target: info.target.name.to_owned(),
                start: *start_time as f64 / 1000.,
                duration,
                rmeta_time,
            })
        }
        let mut concurrency = vec![];
        let mut cpu_usage = vec![];
        let mut active_units = 0;
        for (time, (started, ended)) in unique_times {
            active_units += started;
            active_units -= ended;
            concurrency.push(Concurrency {
                t: time as f64 / 1000.,
                active: active_units,
                waiting: 0,
                inactive: 0,
            });
            cpu_usage.push((
                time as f64 / 1000.,
                active_units as f64 / cores as f64 * 100.,
            ))
        }

        Timings {
            start_str,
            unit_times,
            concurrency,
            cpu_usage,
            total_time,
        }
    }

    /// Save HTML report to disk.
    pub fn report_html(&self, timings_suffix: String) -> Result<()> {
        let timestamp = self.start_str.replace(&['-', ':'][..], "");

        let filename = format!("./cargo-timing-{}-{}.html", timings_suffix, timestamp);
        let file = std::fs::File::create(&filename)?;
        let mut f = BufWriter::new(file);
        f.write_all(HTML_TMPL.as_bytes())?;
        self.write_summary_table(&mut f, self.total_time)?;
        f.write_all(HTML_CANVAS.as_bytes())?;
        // It helps with pixel alignment to use whole numbers.
        writeln!(
            f,
            "<script>\n\
             DURATION = {};",
            f64::ceil(self.total_time) as u32
        )?;
        self.write_js_data(&mut f)?;
        write!(
            f,
            "{}\n\
             </script>\n\
             </body>\n\
             </html>\n\
             ",
            include_str!("timings.js")
        )?;
        drop(f);
        Ok(())
    }

    /// Render the summary table.
    fn write_summary_table(&self, f: &mut impl Write, duration: f64) -> Result<()> {
        let time_human = if duration > 60.0 {
            format!(" ({}m {:.1}s)", duration as u32 / 60, duration % 60.0)
        } else {
            "".to_string()
        };
        let total_time = format!("{:.1}s{}", duration, time_human);
        write!(
            f,
            r#"
<table class="my-table summary-table">
  <tr>
    <td>Build start:</td><td>{}</td>
  </tr>
  <tr>
    <td>Total time:</td><td>{}</td>
  </tr>
</table>
"#,
            self.start_str, total_time,
        )?;
        Ok(())
    }

    /// Write timing data in JavaScript. Primarily for `timings.js` to put data
    /// in a `<script>` HTML element to draw graphs.
    fn write_js_data(&self, f: &mut impl Write) -> Result<()> {
        // Create a map to link indices of unlocked units.
        #[derive(serde::Serialize)]
        struct UnitData {
            i: usize,
            name: String,
            mode: String,
            target: String,
            start: f64,
            duration: f64,
            rmeta_time: Option<f64>,
            unlocked_units: Vec<usize>,
            unlocked_rmeta_units: Vec<usize>,
        }
        let round = |x: f64| (x * 100.0).round() / 100.0;
        let unit_data: Vec<UnitData> = self
            .unit_times
            .iter()
            .enumerate()
            .map(|(i, ut)| {
                let mode = if ut.unit.mode == BuildMode::RunCustomBuild {
                    "run-custom-build"
                } else {
                    "todo"
                }
                .to_string();
                let suffix_start = ut
                    .unit
                    .pkg_id
                    .bytes()
                    .position(|character| character == '(' as u8)
                    .unwrap_or(ut.unit.pkg_id.len());

                UnitData {
                    i,
                    name: ut.unit.pkg_id[..suffix_start].to_owned(),
                    mode,
                    target: "".to_owned(),
                    start: round(ut.start),
                    duration: round(ut.duration),
                    rmeta_time: ut.rmeta_time.map(round),
                    unlocked_units: vec![],
                    unlocked_rmeta_units: vec![],
                }
            })
            .collect();
        writeln!(
            f,
            "const UNIT_DATA = {};",
            serde_json::to_string_pretty(&unit_data)?
        )?;
        writeln!(
            f,
            "const CONCURRENCY_DATA = {};",
            serde_json::to_string_pretty(&self.concurrency)?
        )?;
        writeln!(
            f,
            "const CPU_USAGE = {};",
            serde_json::to_string_pretty(&self.cpu_usage)?
        )?;
        Ok(())
    }
}

impl UnitTime {
    /// Returns the codegen time as (rmeta_time, codegen_time, percent of total)
    fn codegen_time(&self) -> Option<(f64, f64, f64)> {
        self.rmeta_time.map(|rmeta_time| {
            let ctime = self.duration - rmeta_time;
            let cent = (ctime / self.duration) * 100.0;
            (rmeta_time, ctime, cent)
        })
    }

    fn name_ver(&self) -> String {
        self.unit.pkg_id.clone()
    }
}

static HTML_TMPL: &str = r#"
<html>
<head>
  <title>Cargo Build Timings</title>
  <meta charset="utf-8">
<style type="text/css">
html {
  font-family: sans-serif;
}

.canvas-container {
  position: relative;
  margin-top: 5px;
  margin-bottom: 5px;
}

h1 {
  border-bottom: 1px solid #c0c0c0;
}

.graph {
  display: block;
}

.my-table {
  margin-top: 20px;
  margin-bottom: 20px;
  border-collapse: collapse;
  box-shadow: 0 5px 10px rgba(0, 0, 0, 0.1);
}

.my-table th {
  color: #d5dde5;
  background: #1b1e24;
  border-bottom: 4px solid #9ea7af;
  border-right: 1px solid #343a45;
  font-size: 18px;
  font-weight: 100;
  padding: 12px;
  text-align: left;
  vertical-align: middle;
}

.my-table th:first-child {
  border-top-left-radius: 3px;
}

.my-table th:last-child {
  border-top-right-radius: 3px;
  border-right:none;
}

.my-table tr {
  border-top: 1px solid #c1c3d1;
  border-bottom: 1px solid #c1c3d1;
  font-size: 16px;
  font-weight: normal;
}

.my-table tr:first-child {
  border-top:none;
}

.my-table tr:last-child {
  border-bottom:none;
}

.my-table tr:nth-child(odd) td {
  background: #ebebeb;
}

.my-table tr:last-child td:first-child {
  border-bottom-left-radius:3px;
}

.my-table tr:last-child td:last-child {
  border-bottom-right-radius:3px;
}

.my-table td {
  background: #ffffff;
  padding: 10px;
  text-align: left;
  vertical-align: middle;
  font-weight: 300;
  font-size: 14px;
  border-right: 1px solid #C1C3D1;
}

.my-table td:last-child {
  border-right: 0px;
}

.summary-table td:first-child {
  vertical-align: top;
  text-align: right;
}

.input-table td {
  text-align: center;
}

.error-text {
  color: #e80000;
}

</style>
</head>
<body>

<h1>Cargo Build Timings</h1>
See <a href="https://doc.rust-lang.org/nightly/cargo/reference/timings.html">Documentation</a>
"#;

static HTML_CANVAS: &str = r#"
<table class="input-table">
  <tr>
    <td><label for="min-unit-time">Min unit time:</label></td>
    <td><label for="scale">Scale:</label></td>
  </tr>
  <tr>
    <td><input type="range" min="0" max="30" step="0.1" value="0" id="min-unit-time"></td>
    <td><input type="range" min="1" max="50" value="20" id="scale"></td>
  </tr>
  <tr>
    <td><output for="min-unit-time" id="min-unit-time-output"></output></td>
    <td><output for="scale" id="scale-output"></output></td>
  </tr>
</table>

<div id="pipeline-container" class="canvas-container">
 <canvas id="pipeline-graph" class="graph" style="position: absolute; left: 0; top: 0; z-index: 0;"></canvas>
 <canvas id="pipeline-graph-lines" style="position: absolute; left: 0; top: 0; z-index: 1; pointer-events:none;"></canvas>
</div>
<div class="canvas-container">
  <canvas id="timing-graph" class="graph"></canvas>
</div>
"#;
