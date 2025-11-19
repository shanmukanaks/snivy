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

    pub fn current(&mut self) -> Option<f64> {
        if self.window.len() == self.period {
            simple_moving_average(self.window.make_contiguous())
        } else {
            None
        }
    }
}
