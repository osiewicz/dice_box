use std::collections::BTreeMap;

use crate::artifact::Artifact;
use crate::dependency_queue::DependencyQueue;
use crate::timings::TimingInfo;

use log::trace;
use tabled::Tabled;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Duration(std::time::Duration);

impl std::fmt::Display for Duration {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "{:?}", self.0)
    }
}

/// Makespan length, in seconds, of a given schedule.
#[derive(Clone, Debug, PartialEq, PartialOrd, Tabled)]
pub struct Makespan {
    pub label: String,
    pub num_threads: usize,
    pub makespan: Duration,
}

#[derive(Clone, Debug, PartialEq)]
struct Task {
    artifact: Artifact,
    end_time: u64,
}

pub struct Runner {
    current_time: u64,
    queue: DependencyQueue,
    timings: BTreeMap<Artifact, TimingInfo>,
    running_tasks: Vec<Option<Task>>,
    running_tasks_count: usize,
    label: String,
}

impl Runner {
    pub fn new(
        queue: DependencyQueue,
        timings: BTreeMap<Artifact, TimingInfo>,
        num_threads: usize,
    ) -> Self {
        Self {
            running_tasks: vec![None; num_threads],
            label: queue.hints().label(),
            queue,
            timings,
            current_time: 0,
            running_tasks_count: 0,
        }
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = label;
        self
    }
    fn run_next_task_to_completion(&mut self) {
        let mut counter = 0;
        let Some(last_active_task) = self.running_tasks.iter().position(|item| {
            counter += item.is_some() as usize;
            counter == self.running_tasks_count
        }) else {
            // No task is running.
            return;
        };
        let Some(task_to_remove) = self.running_tasks[..=last_active_task]
            .iter()
            .cloned()
            .filter_map(|key| key)
            .min_by_key(|task| task.end_time)
        else {
            return;
        };

        self.running_tasks[..=last_active_task]
            .iter_mut()
            .for_each(|maybe_task| {
                // Clean out any tasks that end at the minimum quantum.
                if let Some(task) = maybe_task.as_ref() {
                    if task.end_time == task_to_remove.end_time {
                        self.running_tasks_count -= 1;
                        let finished = maybe_task.take().unwrap();
                        trace!("Finished {:?}", &finished);
                        let unlocked_units = self.queue.finish(&finished.artifact);
                        if !unlocked_units.is_empty() {
                            trace!("Unlocked units: {:?}", unlocked_units);
                        }
                    }
                }
            });
        self.current_time = task_to_remove.end_time;
    }
    fn busy_slots(&self) -> usize {
        self.running_tasks_count
    }
    fn schedule_new_tasks(&mut self) {
        while let Some(slot_for_task) = self.running_tasks.iter_mut().find(|slot| slot.is_none()) {
            if let Some(new_task) = self.queue.dequeue() {
                trace!("Scheduling {:?}", &new_task);
                *slot_for_task = Some(Task {
                    end_time: self.current_time + (self.timings[&new_task].duration * 1000.) as u64,
                    artifact: new_task,
                });
                self.running_tasks_count += 1;
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
        Makespan {
            label: self.label.clone(),
            num_threads: self.running_tasks.len(),
            makespan: Duration(std::time::Duration::from_millis(self.current_time)),
        }
    }
}
