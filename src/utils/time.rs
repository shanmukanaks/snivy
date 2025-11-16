use chrono::{DateTime, Utc};

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

pub fn interval_to_millis(interval: &str) -> Option<u64> {
    if interval.is_empty() {
        return None;
    }
    let unit = interval.chars().last()?;
    let value_str = &interval[..interval.len().saturating_sub(1)];
    let value: u64 = value_str.parse().ok()?;
    let multiplier = match unit {
        's' | 'S' => 1_000,
        'm' | 'M' => 60_000,
        'h' | 'H' => 3_600_000,
        'd' | 'D' => 86_400_000,
        _ => return None,
    };
    Some(value.saturating_mul(multiplier))
}
