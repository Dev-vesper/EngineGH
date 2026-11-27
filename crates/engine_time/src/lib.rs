//! Fixed-timestep accumulator ("Fix Your Timestep"): simulation advances in constant steps,
//! rendering runs at whatever rate the display allows and interpolates with `alpha`.

/// What to do for one rendered frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepPlan {
    /// Number of fixed simulation steps to run before rendering.
    pub steps: u32,
    /// Position of the render time between the last two simulation states, in [0, 1].
    pub alpha: f32,
    /// Simulation time that had to be discarded this frame because `max_steps` was reached.
    pub dropped_seconds: f64,
}

pub struct FixedTimestep {
    step: f64,
    max_steps: u32,
    accumulator: f64,
}

impl FixedTimestep {
    /// `hz` steps per second; at most `max_steps` steps per frame (spiral-of-death guard).
    pub fn new(hz: f64, max_steps: u32) -> Self {
        assert!(hz > 0.0 && hz.is_finite(), "fixed step rate must be positive");
        assert!(max_steps > 0, "max_steps must be > 0");
        Self { step: 1.0 / hz, max_steps, accumulator: 0.0 }
    }

    /// Fixed step length in seconds.
    #[inline]
    pub fn step_seconds(&self) -> f32 {
        self.step as f32
    }

    /// Adds `frame_dt` seconds of real time and returns how many steps to simulate.
    /// The accumulator is `f64` so long sessions do not drift.
    pub fn advance(&mut self, frame_dt: f64) -> StepPlan {
        self.accumulator += frame_dt.max(0.0);
        let wanted = (self.accumulator / self.step).floor() as u64;
        let steps = wanted.min(self.max_steps as u64) as u32;
        self.accumulator -= steps as f64 * self.step;

        let mut dropped_seconds = 0.0;
        if wanted > steps as u64 {
            // Simulation cannot keep up: drop the whole-step backlog instead of falling ever
            // further behind. Counting steps (not using `%`) keeps this exact at float boundaries.
            let backlog = (wanted - steps as u64) as f64 * self.step;
            self.accumulator = (self.accumulator - backlog).max(0.0);
            dropped_seconds = backlog;
        }
        StepPlan { steps, alpha: ((self.accumulator / self.step) as f32).min(1.0), dropped_seconds }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_multiples_run_exact_steps() {
        let mut t = FixedTimestep::new(50.0, 5); // 0.02 s step
        assert_eq!(t.advance(0.04 + 1e-9).steps, 2);
    }

    #[test]
    fn alpha_is_fraction_of_a_step() {
        let mut t = FixedTimestep::new(100.0, 5); // 10 ms
        let p = t.advance(0.015);
        assert_eq!(p.steps, 1);
        assert!((p.alpha - 0.5).abs() < 1e-4);
    }

    #[test]
    fn small_frames_accumulate() {
        let mut t = FixedTimestep::new(100.0, 5);
        let total: u32 = (0..10).map(|_| t.advance(0.004).steps).sum(); // 40 ms
        assert_eq!(total, 4);
    }

    #[test]
    fn step_count_independent_of_frame_chunking() {
        let run = |dt: f64| {
            let mut t = FixedTimestep::new(60.0, 100);
            (0..(10.0 / dt) as usize).map(|_| t.advance(dt).steps as u64).sum::<u64>()
        };
        let (a, b) = (run(1.0 / 144.0), run(1.0 / 30.0));
        assert!((a as i64 - b as i64).abs() <= 1, "{a} vs {b}");
        assert!((a as i64 - 600).abs() <= 1);
    }

    #[test]
    fn spiral_of_death_is_clamped_and_reported() {
        let mut t = FixedTimestep::new(60.0, 5);
        let p = t.advance(1.0); // one second stall
        assert_eq!(p.steps, 5);
        assert!(p.dropped_seconds > 0.85, "dropped {}", p.dropped_seconds);
        assert!((0.0..=1.0).contains(&p.alpha));
        let next = t.advance(1.0 / 60.0);
        assert!(next.steps <= 1, "backlog must not carry over");
    }

    #[test]
    fn negative_dt_is_ignored() {
        let mut t = FixedTimestep::new(60.0, 5);
        assert_eq!(t.advance(-1.0).steps, 0);
    }
}
