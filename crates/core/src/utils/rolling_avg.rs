use std::collections::VecDeque;

pub struct RollingAvg<T> {
    max_size: usize,
    queue: VecDeque<T>,
    sum: f64,
}

impl<T> RollingAvg<T>
where
    T: Copy + Into<f64>,
{
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            queue: VecDeque::new(),
            sum: 0.0,
        }
    }
    pub fn add(&mut self, value: T) {
        self.queue.push_back(value);
        self.sum += value.into();

        if self.queue.len() > self.max_size
            && let Some(removed) = self.queue.pop_front()
        {
            self.sum -= removed.into();
        }
    }

    pub fn get_avg(&self) -> f64 {
        match self.queue.is_empty() {
            false => self.sum / self.queue.len() as f64,
            true => 0.0,
        }
    }
}
