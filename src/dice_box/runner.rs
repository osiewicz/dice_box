use crate::dependency_queue::{Artifact, DependencyQueue};

/// Makespan length, in seconds, of a given schedule.
pub struct Makespan(pub usize);

/// Whenever Runner has a scheduling decision to make, it will consult it's hint provider.
trait HintProvider {
    fn suggest_next(&self, candidates: &[()]) -> Option<usize> {
        None
    }
}

#[derive(Clone, PartialEq)]
struct Task {
    artifact: (),
    end_time: usize,
}

pub struct Runner {
    current_time: usize,
    hints: Box<dyn HintProvider>,
    queue: DependencyQueue<(), Artifact, ()>,
    running_tasks: Vec<Option<Task>>,
}

impl Runner {
    fn new(hints: Box<dyn HintProvider>, num_threads: usize) -> Self {
        Self {
            hints,
            running_tasks: vec![None; num_threads],
            queue: DependencyQueue::new(),
            current_time: 0,
        }
    }

    fn run_next_task_to_completion(&mut self) {
        let task_to_remove = self
            .running_tasks
            .iter()
            .min_by_key(|task| task.as_ref().and_then(|t| Some(t.end_time)))
            .cloned()
            .flatten()
            .expect("There must be at least one task");
        self.running_tasks.retain_mut(|maybe_task| {
            // Clean out any tasks that end at the minimum quantum.
            if let Some(task) = maybe_task.as_ref() {
                if task == &task_to_remove {
                    maybe_task.take();
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
            if let Some((new_task, _, _)) = self.queue.dequeue() {
                let slot_for_task = self
                    .running_tasks
                    .iter_mut()
                    .find(|slot| slot.is_none())
                    .expect("There should be at least one empty slot in the queue at this point, as we wouldn't be in the loop otherwise.");
                *slot_for_task = Some(Task {
                    artifact: new_task,
                    end_time: self.current_time + 0,
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
