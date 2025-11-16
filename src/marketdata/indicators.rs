use std::collections::VecDeque;

use crate::utils::math::simple_moving_average;

#[derive(Debug, Clone)]
pub struct MovingAverage {
    window: VecDeque<f64>,
    period: usize,
}

impl MovingAverage {
    pub fn new(period: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(period),
            period,
        }
    }

    pub fn seed(&mut self, values: &[f64]) {
        self.window.clear();
        for value in values.iter().cloned().take(self.period) {
            self.window.push_back(value);
        }
    }

    pub fn update(&mut self, value: f64) -> Option<f64> {
        self.window.push_back(value);
        if self.window.len() > self.period {
            self.window.pop_front();
        }
        if self.window.len() == self.period {
            simple_moving_average(self.window.make_contiguous())
        } else {
            None
        }
    }

    pub fn current(&self) -> Option<f64> {
        if self.window.len() == self.period {
            Some(self.window.iter().sum::<f64>() / self.period as f64)
        } else {
            None
        }
    }

    pub fn values(&self) -> Vec<f64> {
        self.window.iter().cloned().collect()
    }

    pub fn is_ready(&self) -> bool {
        if self.window.len() == self.period {
            true
        } else {
            false
        }
    }
}
