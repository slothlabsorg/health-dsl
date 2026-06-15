//! The aggregated [`HealthReport`] and the [`aggregate`] function backing it.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::check::CheckOutcome;
use crate::status::Status;

/// Aggregate per-check outcomes into an overall [`Status`]:
///  - a **critical** check that is [`Status::Down`] makes the system
///    [`Status::Down`];
///  - a **non-critical** [`Status::Down`], or any [`Status::Degraded`], degrades
///    it;
///  - otherwise the system is [`Status::Up`] (including when there are no
///    checks).
pub(crate) fn aggregate(outcomes: &[CheckOutcome]) -> Status {
    let mut overall = Status::Up;
    for o in outcomes {
        let effective = match o.status {
            Status::Down if o.critical => Status::Down,
            Status::Down => Status::Degraded,
            Status::Degraded => Status::Degraded,
            Status::Up => Status::Up,
        };
        overall = overall.worst(effective);
    }
    overall
}

/// The aggregated result of running a [`HealthRegistry`](crate::HealthRegistry).
///
/// Derives [`Serialize`], so `serde_json::to_string(&report)` and friends work
/// directly; [`HealthReport::to_json`] renders the same compact, keyed shape the
/// Kotlin original produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthReport {
    /// Overall system status.
    pub status: Status,
    /// Per-check outcomes, in declaration order.
    #[serde(serialize_with = "serialize_checks")]
    pub checks: Vec<CheckOutcome>,
    /// Wall-clock time to run all checks. They run concurrently, so this is
    /// roughly the slowest check, not their sum.
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

impl HealthReport {
    /// True unless the system is [`Status::Down`]; suitable for a readiness
    /// gate.
    pub fn is_healthy(&self) -> bool {
        self.status != Status::Down
    }

    /// Render the report as a compact JSON document.
    ///
    /// The shape is a `status`, a `durationMs`, and a `checks` object keyed by
    /// check name — each value carrying `status`, `critical`, `durationMs`, and
    /// (when present) `message` and `details`.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("HealthReport always serializes")
    }

    /// Build the same nested structure [`to_json`](Self::to_json) renders, as a
    /// [`serde_json::Value`] — convenient for embedding in a larger document or
    /// handing to another serializer.
    pub fn to_value(&self) -> Value {
        let mut root = Map::new();
        root.insert("status".into(), Value::from(self.status.as_str()));
        root.insert("durationMs".into(), Value::from(self.duration_ms));
        root.insert("checks".into(), Value::Object(checks_object(&self.checks)));
        Value::Object(root)
    }
}

/// Serialize the `checks` vector as an object keyed by check name, matching the
/// reference JSON shape rather than a flat array.
fn serialize_checks<S>(checks: &[CheckOutcome], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    checks_object(checks).serialize(serializer)
}

fn checks_object(checks: &[CheckOutcome]) -> Map<String, Value> {
    let mut obj = Map::new();
    for o in checks {
        let mut entry = Map::new();
        entry.insert("status".into(), Value::from(o.status.as_str()));
        entry.insert("critical".into(), Value::from(o.critical));
        entry.insert("durationMs".into(), Value::from(o.duration_ms));
        if let Some(message) = &o.message {
            entry.insert("message".into(), Value::from(message.clone()));
        }
        if !o.details.is_empty() {
            entry.insert("details".into(), details_value(&o.details));
        }
        obj.insert(o.name.clone(), Value::Object(entry));
    }
    obj
}

fn details_value(details: &BTreeMap<String, String>) -> Value {
    let mut map = Map::new();
    for (k, v) in details {
        map.insert(k.clone(), Value::from(v.clone()));
    }
    Value::Object(map)
}
