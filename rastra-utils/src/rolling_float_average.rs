use std::collections::VecDeque;

pub struct RollingFloatAverage {
    max_size: usize,
    queue: VecDeque<f64>,
    sum: f64,
}

impl RollingFloatAverage {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            queue: VecDeque::new(),
            sum: 0.0,
        }
    }
    pub fn add(&mut self, value: f64) {
        self.queue.push_back(value);
        self.sum += value;

        if self.queue.len() > self.max_size {
            if let Some(removed) = self.queue.pop_front() {
                self.sum -= removed;
            }
        }
    }

    pub fn get_avg(&self) -> f64 {
        match self.queue.is_empty() {
            false => self.sum / self.queue.len() as f64,
            true => 0.0,
        }
    }
}