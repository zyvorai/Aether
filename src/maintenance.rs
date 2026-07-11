// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Interval job scheduler for the `aether serve` maintenance loop.
//!
//! Generalizes the previously-hardcoded 60s maintenance tick into a registry of
//! named jobs, each with its own interval. The scheduler owns only the *timing*
//! (a fixed tick advances per-job elapsed counters and reports which jobs are
//! due) so the async work stays in the serve loop where it can borrow shared
//! state. Keeping timing pure makes the cadence unit-testable without a runtime.
//!
//! A job whose interval is `<= tick_secs` runs on every tick. Elapsed time
//! starts at zero, so a job first fires after one full interval has passed
//! (matching the original scheduled-backup behavior).
//!
//! Distinct from [`crate::scheduler`], which is the *workload placement* engine.

/// A registered job's timing state.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Job {
    name: String,
    interval_secs: u64,
    elapsed_secs: u64,
}

/// Fixed-tick interval scheduler. Not thread-safe by itself; the serve loop owns
/// a single instance and drives it from one task.
#[derive(Debug, Clone)]
pub struct JobScheduler {
    tick_secs: u64,
    jobs: Vec<Job>,
}

impl JobScheduler {
    /// Create a scheduler with the given tick granularity (seconds, minimum 1).
    pub fn new(tick_secs: u64) -> Self {
        Self {
            tick_secs: tick_secs.max(1),
            jobs: Vec::new(),
        }
    }

    /// The tick granularity in seconds.
    pub fn tick_secs(&self) -> u64 {
        self.tick_secs
    }

    /// Register a job. `interval_secs <= tick_secs` runs every tick.
    /// Re-registering the same name replaces the prior interval and resets its
    /// elapsed counter.
    pub fn register(&mut self, name: &str, interval_secs: u64) -> &mut Self {
        if let Some(existing) = self.jobs.iter_mut().find(|j| j.name == name) {
            existing.interval_secs = interval_secs;
            existing.elapsed_secs = 0;
        } else {
            self.jobs.push(Job {
                name: name.to_string(),
                interval_secs,
                elapsed_secs: 0,
            });
        }
        self
    }

    /// Names of currently-registered jobs, in registration order.
    pub fn job_names(&self) -> Vec<String> {
        self.jobs.iter().map(|j| j.name.clone()).collect()
    }

    /// Whether any job is registered.
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Advance every job by one tick and return the names of jobs now due, in
    /// registration order. Due jobs have their elapsed counter reset.
    pub fn tick(&mut self) -> Vec<String> {
        let tick = self.tick_secs;
        let mut due = Vec::new();
        for job in &mut self.jobs {
            job.elapsed_secs = job.elapsed_secs.saturating_add(tick);
            // interval <= tick ⇒ due every tick; otherwise once elapsed catches up.
            if job.interval_secs <= tick || job.elapsed_secs >= job.interval_secs {
                due.push(job.name.clone());
                job.elapsed_secs = 0;
            }
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tick_job_fires_each_tick() {
        let mut s = JobScheduler::new(60);
        s.register("webhook", 0);
        assert_eq!(s.tick(), vec!["webhook"]);
        assert_eq!(s.tick(), vec!["webhook"]);
    }

    #[test]
    fn interval_job_fires_after_interval() {
        let mut s = JobScheduler::new(60);
        s.register("backup", 180); // every 3 ticks
        assert!(s.tick().is_empty()); // 60
        assert!(s.tick().is_empty()); // 120
        assert_eq!(s.tick(), vec!["backup"]); // 180 → due, reset
        assert!(s.tick().is_empty()); // 60
        assert!(s.tick().is_empty()); // 120
        assert_eq!(s.tick(), vec!["backup"]); // 180 → due again
    }

    #[test]
    fn interval_shorter_than_tick_runs_every_tick() {
        let mut s = JobScheduler::new(60);
        s.register("cert", 30); // shorter than tick
        assert_eq!(s.tick(), vec!["cert"]);
        assert_eq!(s.tick(), vec!["cert"]);
    }

    #[test]
    fn multiple_jobs_return_in_registration_order() {
        let mut s = JobScheduler::new(60);
        s.register("a", 0);
        s.register("b", 120);
        s.register("c", 0);
        assert_eq!(s.tick(), vec!["a", "c"]); // b not yet due
        assert_eq!(s.tick(), vec!["a", "b", "c"]); // b due at 120
    }

    #[test]
    fn non_aligned_interval_fires_once_interval_elapsed() {
        let mut s = JobScheduler::new(60);
        s.register("j", 100); // not a multiple of tick
        assert!(s.tick().is_empty()); // 60 < 100
        assert_eq!(s.tick(), vec!["j"]); // 120 >= 100 → due, reset
        assert!(s.tick().is_empty()); // 60
        assert_eq!(s.tick(), vec!["j"]); // 120 → due
    }

    #[test]
    fn reregister_replaces_interval_and_resets() {
        let mut s = JobScheduler::new(60);
        s.register("j", 120);
        assert!(s.tick().is_empty()); // 60
        s.register("j", 0); // now every tick, elapsed reset
        assert_eq!(s.tick(), vec!["j"]);
        assert_eq!(s.job_names(), vec!["j"]);
    }

    #[test]
    fn tick_secs_floor_is_one() {
        let s = JobScheduler::new(0);
        assert_eq!(s.tick_secs(), 1);
    }
}
