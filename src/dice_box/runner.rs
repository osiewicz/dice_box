/// Makespan length, in seconds, of a given schedule.
pub struct Makespan(pub f64);

pub fn run(num_threads: u64) -> Makespan {
    Makespan(0.)
}

/// Whenever Runner has a scheduling decision to make, it will consult it's hint provider.
trait HintProvider {
    fn suggest_next(&self, candidates: &[()]) -> Option<usize> {
        None
    }
}

struct Runner {
    hints: Box<dyn HintProvider>,
    num_threads: u64,
}

impl Runner {
    fn new(hints: Box<dyn HintProvider>, num_threads: u64) -> Self {
        Self { hints, num_threads }
    }
}
