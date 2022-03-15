use std::time::Duration;
use std::time::Instant;

pub mod candidacy;
pub mod evaluate;
pub mod greedy;
pub mod pst;
pub mod search;
pub mod transposition;

pub trait TimeoutCondition {
    /// Determine whether the timeout condition is over.
    fn is_over(&self) -> bool;

    /// Start the timeout condition.
    fn start(&mut self);
}

pub struct NoTimeout;

impl TimeoutCondition for NoTimeout {
    fn is_over(&self) -> bool {
        false
    }

    fn start(&mut self) {}
}

pub struct ElapsedTimeout {
    start: Instant,
    duration: Duration,
}

impl ElapsedTimeout {
    #[allow(unused)]
    /// Create a new elapsed timeout with a default start of right now.
    pub fn new(d: Duration) -> ElapsedTimeout {
        ElapsedTimeout {
            start: Instant::now(),
            duration: d,
        }
    }
}

impl TimeoutCondition for ElapsedTimeout {
    fn is_over(&self) -> bool {
        Instant::now() - self.duration > self.start
    }

    fn start(&mut self) {
        self.start = Instant::now();
    }
}
