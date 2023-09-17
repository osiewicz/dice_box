use std::collections::BTreeMap;

use crate::artifact::{Artifact, ArtifactType};
use crate::dependency_queue::DependencyQueue;
use crate::timings::TimingInfo;

/// Makespan length, in seconds, of a given schedule.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Makespan(pub usize);

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

#[derive(Debug)]
struct AggregateHintProvider(Vec<Box<dyn HintProvider>>);

impl HintProvider for AggregateHintProvider {
    fn suggest_next<'a>(&self, timings: &[&'a Artifact]) -> Option<&'a Artifact> {
        self.0
            .iter()
            .find_map(|provider| provider.suggest_next(timings))
    }
}
#[derive(Clone, Debug, PartialEq)]
struct Task {
    artifact: Artifact,
    end_time: usize,
}

pub struct Runner {
    current_time: usize,
    queue: DependencyQueue,
    timings: BTreeMap<Artifact, TimingInfo>,
    running_tasks: Vec<Option<Task>>,
}

impl Runner {
    pub fn new(
        queue: DependencyQueue,
        timings: BTreeMap<Artifact, TimingInfo>,
        num_threads: usize,
    ) -> Self {
        Self {
            running_tasks: vec![None; num_threads],
            queue,
            timings,
            current_time: 0,
        }
    }

    fn run_next_task_to_completion(&mut self) {
        let Some(task_to_remove) = self
            .running_tasks
            .iter()
            .filter(|f| f.is_some())
            .min_by_key(|task| task.as_ref().and_then(|t| Some(t.end_time)))
            .cloned()
            .flatten()
        else {
            return;
        };
        self.running_tasks.retain_mut(|maybe_task| {
            // Clean out any tasks that end at the minimum quantum.
            if let Some(task) = maybe_task.as_ref() {
                if task == &task_to_remove {
                    let finished = maybe_task.take();
                    self.queue.finish(&finished.unwrap().artifact);
                }
            }
            true
        });
        self.current_time = task_to_remove.end_time;
    }
    fn free_slots(&self) -> usize {
        self.running_tasks
            .iter()
            .filter(|task| task.is_none())
            .count()
    }
    fn busy_slots(&self) -> usize {
        self.running_tasks.len() - self.free_slots()
    }
    fn schedule_new_tasks(&mut self) {
        while self.free_slots() > 0 {
            if let Some(new_task) = self.queue.dequeue() {
                let slot_for_task = self
                    .running_tasks
                    .iter_mut()
                    .find(|slot| slot.is_none())
                    .expect("There should be at least one empty slot in the queue at this point, as we wouldn't be in the loop otherwise.");
                *slot_for_task = Some(Task {
                    end_time: self.current_time
                        + (self.timings[&new_task].duration * 100.) as usize,
                    artifact: new_task,
                });
            } else {
                break;
            }
        }
    }
    fn step(&mut self) {
        self.run_next_task_to_completion();
        self.schedule_new_tasks();
    }
    pub fn calculate(&mut self) -> Makespan {
        while !self.queue.is_empty() || self.busy_slots() > 0 {
            self.step();
        }
        assert_eq!(self.busy_slots(), 0);
        Makespan(self.current_time)
    }
}
