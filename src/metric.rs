use std::time::{Duration, Instant};

pub struct Metric {
    pub id: String,
    pub runtime: String,
    pub status: String,
    pub start: Instant,
    pub end: Instant,
    pub elapsed: Duration,
    pub consumed: Duration,
}
