//! A HintProvider that parses build order and then suggests builds in the same order.
//! This is useful mostly for testing and calculating the communication overhead of a true cargo build process, where
//! communication costs are non-zero.
use std::collections::VecDeque;

use itertools::Itertools;

use crate::artifact::Artifact;

use super::HintProvider;

#[derive(Debug)]
pub struct RepeatSchedule(VecDeque<Artifact>);

impl RepeatSchedule {
    pub fn new(artifacts: Vec<Artifact>) -> Box<dyn HintProvider> {
        // Deduplicate entires.
        Box::new(Self(artifacts.into_iter().unique().collect()))
    }
}

impl HintProvider for RepeatSchedule {
    fn suggest_next<'a>(&mut self, timings: &[&'a Artifact]) -> Option<&'a Artifact> {
        let next = self.0.front()?;
        let item = timings.iter().find(|item| **item == next)?;
        self.0.pop_front();
        Some(item)
    }
}
