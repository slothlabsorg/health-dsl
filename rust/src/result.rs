//! The [`CheckResult`] a check body returns.

use std::collections::BTreeMap;

use crate::status::Status;

/// The outcome a check body returns.
///
/// Construct it with the associated functions ([`CheckResult::up`],
/// [`CheckResult::up_with`], [`CheckResult::degraded`], [`CheckResult::down`])
/// so a body reads as a decision table:
///
/// ```
/// use health_dsl::CheckResult;
///
/// # fn free_percent() -> u8 { 12 }
/// let free = free_percent();
/// let result = if free < 5 {
///     CheckResult::down(format!("disk almost full: {free}%"))
/// } else if free < 15 {
///     CheckResult::degraded(format!("disk low: {free}%"))
/// } else {
///     CheckResult::up_with([("freePercent", free.to_string())])
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// The health this body is reporting.
    pub status: Status,
    /// An optional human-readable explanation (set for degraded/down).
    pub message: Option<String>,
    /// Arbitrary string key/value detail, ordered for stable serialization.
    pub details: BTreeMap<String, String>,
}

impl CheckResult {
    /// The dependency is fully operational, with no extra detail.
    pub fn up() -> CheckResult {
        CheckResult {
            status: Status::Up,
            message: None,
            details: BTreeMap::new(),
        }
    }

    /// The dependency is fully operational, carrying some detail.
    ///
    /// ```
    /// use health_dsl::CheckResult;
    /// let r = CheckResult::up_with([("hitRate", "0.93".to_string())]);
    /// assert_eq!(r.details["hitRate"], "0.93");
    /// ```
    pub fn up_with<I, K>(details: I) -> CheckResult
    where
        I: IntoIterator<Item = (K, String)>,
        K: Into<String>,
    {
        CheckResult {
            status: Status::Up,
            message: None,
            details: into_details(details),
        }
    }

    /// The dependency is impaired but the service can still serve.
    pub fn degraded(message: impl Into<String>) -> CheckResult {
        CheckResult {
            status: Status::Degraded,
            message: Some(message.into()),
            details: BTreeMap::new(),
        }
    }

    /// The dependency is impaired, carrying some detail.
    pub fn degraded_with<I, K>(message: impl Into<String>, details: I) -> CheckResult
    where
        I: IntoIterator<Item = (K, String)>,
        K: Into<String>,
    {
        CheckResult {
            status: Status::Degraded,
            message: Some(message.into()),
            details: into_details(details),
        }
    }

    /// The dependency is unusable.
    pub fn down(message: impl Into<String>) -> CheckResult {
        CheckResult {
            status: Status::Down,
            message: Some(message.into()),
            details: BTreeMap::new(),
        }
    }

    /// The dependency is unusable, carrying some detail.
    pub fn down_with<I, K>(message: impl Into<String>, details: I) -> CheckResult
    where
        I: IntoIterator<Item = (K, String)>,
        K: Into<String>,
    {
        CheckResult {
            status: Status::Down,
            message: Some(message.into()),
            details: into_details(details),
        }
    }
}

fn into_details<I, K>(details: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = (K, String)>,
    K: Into<String>,
{
    details.into_iter().map(|(k, v)| (k.into(), v)).collect()
}
