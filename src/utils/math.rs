pub fn simple_moving_average(window: &[f64]) -> Option<f64> {
    if window.is_empty() {
        None
    } else {
        Some(window.iter().sum::<f64>() / window.len() as f64)
    }
}

pub fn format_decimal(mut value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if value.is_sign_negative() && value.abs() < f64::EPSILON {
        value = 0.0;
    }
    let mut out = format!("{:.8}", value);
    while out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    if out.is_empty() || out == "-0" {
        "0".to_string()
    } else {
        out
    }
}
