use std::time::{Duration, Instant};

pub struct Every {
    max: Duration,
    start: Instant,
}

impl Every {
    pub fn new(max: Duration) -> Self {
        Self {
            max,
            start: Instant::now(),
        }
    }
}

impl Iterator for Every {
    type Item = ();
    fn next(&mut self) -> Option<Self::Item> {
        let elapsed = self.start.elapsed();
        if elapsed < self.max {
            std::thread::sleep(self.max - elapsed);
        }

        self.start = Instant::now();

        Some(())
    }
}
