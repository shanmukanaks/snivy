pub fn simple_moving_average(window: &[f64]) -> Option<f64> {
    if window.is_empty() {
        None
    } else {
        Some(window.iter().sum::<f64>() / window.len() as f64)
    }
}
