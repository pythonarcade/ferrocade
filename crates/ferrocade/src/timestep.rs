//! Fixed-timestep accumulator: decides how many `on_update` calls run before each draw.

pub struct FixedTimestep {
    step: f64,
    accumulator: f64,
    max_steps_per_frame: u32,
}

impl FixedTimestep {
    pub fn new(step: f64) -> Self {
        Self {
            step,
            accumulator: 0.0,
            max_steps_per_frame: 5,
        }
    }

    pub fn step(&self) -> f64 {
        self.step
    }

    /// Add `elapsed` real seconds and return how many updates to run now.
    ///
    /// If the game falls more than `max_steps_per_frame` behind, the excess time is discarded
    /// so a slow frame can't cause an ever-growing backlog of updates.
    pub fn advance(&mut self, elapsed: f64) -> u32 {
        self.accumulator += elapsed.max(0.0);
        let mut steps = 0;
        while self.accumulator >= self.step && steps < self.max_steps_per_frame {
            self.accumulator -= self.step;
            steps += 1;
        }
        if steps == self.max_steps_per_frame && self.accumulator >= self.step {
            self.accumulator = 0.0;
        }
        steps
    }
}
