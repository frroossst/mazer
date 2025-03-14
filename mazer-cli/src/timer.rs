use std::time::Instant;

pub struct Timer(Option<Instant>);

impl Timer {
    pub fn new() -> Self {
        Self {
            0: None,
        }
    }

    pub fn start(&mut self) {
        self.0 = Some(std::time::Instant::now());
    }

    pub fn stop(&mut self) -> u128 {
        match self.0 {
            Some(start) => {
                let end = std::time::Instant::now();
                let duration = end.duration_since(start);
                duration.as_millis()
            }
            None => 0,
        }
    }

    pub fn reset(&mut self) {
        self.0 = None;
    }
}
