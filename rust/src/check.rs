//! A single named [`Check`] and the [`CheckOutcome`] of running it.

use std::collections::BTreeMap;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::time::{Duration, Instant};

use futures::FutureExt;

use crate::result::CheckResult;
use crate::status::Status;

/// Whether a check is critical to the service's readiness.
///
/// A clearer-at-the-call-site alternative to a bare `bool`:
///
/// ```
/// use health_dsl::Critical;
/// assert_eq!(Critical::Yes.is_critical(), true);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Critical {
    /// A [`Status::Down`] result fails the whole report.
    Yes,
    /// A [`Status::Down`] result only degrades the report.
    No,
}

impl Critical {
    /// Whether this marks the check as critical.
    pub fn is_critical(self) -> bool {
        matches!(self, Critical::Yes)
    }
}

impl From<bool> for Critical {
    fn from(value: bool) -> Self {
        if value {
            Critical::Yes
        } else {
            Critical::No
        }
    }
}

/// The boxed future a check body produces.
pub(crate) type CheckFuture = Pin<Box<dyn Future<Output = CheckResult> + Send>>;

/// The boxed, callable check body. Each invocation produces a fresh future, so
/// a registry can be [`run`](crate::HealthRegistry::run) more than once.
pub(crate) type CheckBody = Box<dyn Fn() -> CheckFuture + Send + Sync>;

/// A single named health check. Created via the
/// [`builder`](crate::HealthRegistry::builder), not directly.
pub(crate) struct Check {
    /// Unique, human-readable identifier.
    pub name: String,
    /// When critical, a [`Status::Down`] result fails the whole report; when
    /// not, it only degrades it.
    pub critical: bool,
    /// The check is considered [`Status::Down`] if it does not finish within
    /// this duration.
    pub timeout: Duration,
    /// The check body.
    pub body: CheckBody,
}

impl Check {
    /// Run the check, never failing: timeouts and panics are folded into a
    /// [`Status::Down`] outcome so one misbehaving check cannot break the
    /// report.
    pub async fn execute(&self) -> CheckOutcome {
        let start = Instant::now();
        let result = run_body(&self.body, self.timeout).await;
        let duration_ms = start.elapsed().as_millis() as u64;
        CheckOutcome {
            name: self.name.clone(),
            critical: self.critical,
            status: result.status,
            message: result.message,
            details: result.details,
            duration_ms,
        }
    }
}

/// Drive a check body to a [`CheckResult`], converting a timeout or a panic
/// into a [`Status::Down`] outcome.
async fn run_body(body: &CheckBody, timeout: Duration) -> CheckResult {
    // `catch_unwind` requires `UnwindSafe`; a check body's future generally is
    // not, but turning a panic into DOWN cannot leave *our* state inconsistent
    // (we own nothing the future borrows), so asserting it is sound here.
    let guarded = AssertUnwindSafe(body()).catch_unwind();
    match tokio::time::timeout(timeout, guarded).await {
        Err(_elapsed) => CheckResult::down(format!("timed out after {}ms", timeout.as_millis())),
        Ok(Ok(result)) => result,
        Ok(Err(panic)) => CheckResult::down(panic_message(panic)),
    }
}

/// Extract a human-readable message from a caught panic payload.
fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "panicked".to_string()
    }
}

/// The recorded result of running a [`Check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    /// The check's unique name.
    pub name: String,
    /// Whether the check is critical to readiness.
    pub critical: bool,
    /// The status the check reported (or `DOWN` for a timeout/panic).
    pub status: Status,
    /// An optional explanation.
    pub message: Option<String>,
    /// Any detail the check carried.
    pub details: BTreeMap<String, String>,
    /// Wall-clock time the check took, in milliseconds.
    pub duration_ms: u64,
}
