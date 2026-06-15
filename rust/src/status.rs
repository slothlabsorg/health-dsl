//! The [`Status`] of a single check or of the system as a whole.

use serde::Serialize;

/// The health of a single check or of the system as a whole.
///
/// Ordered from healthiest to least healthy so that aggregation can simply take
/// the "worst" of a set: [`Status::Up`] < [`Status::Degraded`] < [`Status::Down`].
/// The [`Ord`] derive relies on this declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    /// Fully operational.
    Up,
    /// Serving, but impaired — a non-critical dependency is unhealthy.
    Degraded,
    /// Not serving — a critical dependency is unhealthy.
    Down,
}

impl Status {
    /// The wire name of the status (`"UP"`, `"DEGRADED"`, `"DOWN"`), matching
    /// the JSON serialization.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Up => "UP",
            Status::Degraded => "DEGRADED",
            Status::Down => "DOWN",
        }
    }

    /// Return the less-healthy of two statuses.
    pub(crate) fn worst(self, other: Status) -> Status {
        self.max(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_up_degraded_down() {
        assert!(Status::Up < Status::Degraded);
        assert!(Status::Degraded < Status::Down);
        assert!(Status::Up < Status::Down);
    }

    #[test]
    fn worst_takes_the_least_healthy() {
        assert_eq!(Status::Up.worst(Status::Degraded), Status::Degraded);
        assert_eq!(Status::Down.worst(Status::Up), Status::Down);
        assert_eq!(Status::Degraded.worst(Status::Degraded), Status::Degraded);
    }
}
