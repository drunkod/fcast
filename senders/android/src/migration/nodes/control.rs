use crate::migration::protocol::{ControlMode, ControlPoint};
use chrono::{DateTime, Utc};
use serde_json::Value;

fn interpolate_values(start: &Value, end: &Value, ratio: f64) -> Option<Value> {
    let start = start.as_f64()?;
    let end = end.as_f64()?;
    Some(Value::from(start + (end - start) * ratio))
}

/// Evaluates control points at `at` using old-node semantics:
/// - `set`: use the latest point at/before time.
/// - `interpolate`: linearly interpolate to the next point when both are numeric.
pub fn evaluate_control_points(points: &[ControlPoint], at: DateTime<Utc>) -> Option<Value> {
    if points.is_empty() {
        return None;
    }

    let mut before: Option<&ControlPoint> = None;
    let mut after: Option<&ControlPoint> = None;
    for point in points {
        if point.time <= at {
            if before.is_none_or(|candidate| point.time >= candidate.time) {
                before = Some(point);
            }
        } else if after.is_none_or(|candidate| point.time < candidate.time) {
            after = Some(point);
        }
    }

    let Some(current) = before.or(after) else {
        return None;
    };

    if current.mode == ControlMode::Interpolate {
        if let Some(next) = after {
            let total_ms = (next.time - current.time).num_milliseconds();
            if total_ms > 0 {
                let elapsed_ms = (at - current.time).num_milliseconds().max(0);
                let ratio = (elapsed_ms as f64 / total_ms as f64).clamp(0.0, 1.0);
                if let Some(value) = interpolate_values(&current.value, &next.value, ratio) {
                    return Some(value);
                }
            }
        }
    }

    Some(current.value.clone())
}
