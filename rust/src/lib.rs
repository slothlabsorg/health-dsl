//! # health-dsl
//!
//! **A tiny async DSL for declaring service readiness/liveness checks — run
//! concurrently, aggregated correctly, serialized anywhere.**
//!
//! Most services grow an ad-hoc `/health` endpoint: a pile of `match`/`?`
//! blocks, inconsistent timeouts, one slow dependency that hangs the whole
//! probe, and no clear distinction between "we're down" and "we're degraded but
//! still serving." `health-dsl` makes that a declaration:
//!
//! ```
//! use std::time::Duration;
//! use health_dsl::{CheckResult, Critical, HealthRegistry};
//!
//! # async fn example() -> Result<(), health_dsl::BuildError> {
//! let registry = HealthRegistry::builder()
//!     .check("database", Critical::Yes, Duration::from_secs(5), || async {
//!         // ... ping the database ...
//!         CheckResult::up()
//!     })
//!     .check_default("cache", || async {
//!         CheckResult::up_with([("hitRate", "0.93".to_string())])
//!     })
//!     .build()?;
//!
//! let report = registry.run().await; // all checks run concurrently
//! assert!(report.is_healthy());
//! println!("{}", report.to_json());
//! # Ok(())
//! # }
//! ```
//!
//! ## What it gives you
//!
//! - **Correct aggregation.** A `critical` dependency that is `DOWN` makes the
//!   system `DOWN`; a non-critical failure (or any `degraded`) makes it
//!   `DEGRADED` but still healthy.
//! - **Concurrency + per-check timeouts** via `tokio`. `report.duration_ms` is
//!   roughly your slowest check, not the sum. A hung dependency becomes a `DOWN`
//!   outcome after its `timeout` instead of hanging the probe.
//! - **No check can break the report.** Timeouts and panics are folded into a
//!   `DOWN` outcome.
//! - **Serialize anywhere.** [`HealthReport`] derives [`serde::Serialize`] and
//!   offers [`HealthReport::to_json`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod check;
mod report;
mod result;
mod status;

use std::collections::HashSet;
use std::future::Future;
use std::time::{Duration, Instant};

use futures::future::join_all;

use crate::check::Check;

pub use crate::check::{CheckOutcome, Critical};
pub use crate::report::HealthReport;
pub use crate::result::CheckResult;
pub use crate::status::Status;

/// Default per-check timeout when none is given.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// The error returned by [`HealthRegistryBuilder::build`] when the declared
/// checks are invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// Two checks were declared with the same name.
    DuplicateName(String),
    /// A check was declared with a blank name.
    BlankName,
    /// A check was declared with a zero (or negative-equivalent) timeout.
    NonPositiveTimeout(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::DuplicateName(name) => write!(f, "duplicate check name: {name}"),
            BuildError::BlankName => write!(f, "check name must not be blank"),
            BuildError::NonPositiveTimeout(name) => {
                write!(f, "timeout for '{name}' must be positive")
            }
        }
    }
}

impl std::error::Error for BuildError {}

/// An immutable set of health checks. Obtain one from
/// [`HealthRegistry::builder`] and call [`run`](Self::run) to execute every
/// check concurrently and aggregate a [`HealthReport`].
pub struct HealthRegistry {
    checks: Vec<Check>,
}

impl std::fmt::Debug for HealthRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthRegistry")
            .field("names", &self.names())
            .finish()
    }
}

impl HealthRegistry {
    /// Start building a registry.
    pub fn builder() -> HealthRegistryBuilder {
        HealthRegistryBuilder::new()
    }

    /// The names of all registered checks, in declaration order.
    pub fn names(&self) -> Vec<&str> {
        self.checks.iter().map(|c| c.name.as_str()).collect()
    }

    /// Run every check concurrently (each with its own timeout) and aggregate
    /// the results. Resolves once all checks complete. Never fails for a check
    /// failure — failures become [`Status::Down`] outcomes.
    pub async fn run(&self) -> HealthReport {
        let start = Instant::now();
        let outcomes = join_all(self.checks.iter().map(|check| check.execute())).await;
        let duration_ms = start.elapsed().as_millis() as u64;
        HealthReport {
            status: report::aggregate(&outcomes),
            checks: outcomes,
            duration_ms,
        }
    }
}

/// Builder for a [`HealthRegistry`]. Chain [`check`](Self::check) /
/// [`check_default`](Self::check_default) calls, then [`build`](Self::build).
///
/// Validation (blank names, duplicate names, non-positive timeouts) is deferred
/// to [`build`](Self::build) so the chain stays infallible and fluent.
pub struct HealthRegistryBuilder {
    checks: Vec<Check>,
}

impl HealthRegistryBuilder {
    fn new() -> Self {
        HealthRegistryBuilder { checks: Vec::new() }
    }

    /// Declare a check.
    ///
    /// `body` is a closure producing an `async` block (a fresh future on every
    /// run) that resolves to a [`CheckResult`].
    ///
    /// ```
    /// use std::time::Duration;
    /// use health_dsl::{CheckResult, Critical, HealthRegistry};
    ///
    /// let builder = HealthRegistry::builder().check(
    ///     "database",
    ///     Critical::Yes,
    ///     Duration::from_secs(2),
    ///     || async { CheckResult::down("unreachable") },
    /// );
    /// ```
    pub fn check<F, Fut>(
        mut self,
        name: impl Into<String>,
        critical: impl Into<Critical>,
        timeout: Duration,
        body: F,
    ) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CheckResult> + Send + 'static,
    {
        self.checks.push(Check {
            name: name.into(),
            critical: critical.into().is_critical(),
            timeout,
            body: Box::new(move || Box::pin(body())),
        });
        self
    }

    /// Declare a non-critical check with the [`DEFAULT_TIMEOUT`].
    ///
    /// ```
    /// use health_dsl::{CheckResult, HealthRegistry};
    ///
    /// let builder = HealthRegistry::builder()
    ///     .check_default("cache", || async { CheckResult::up() });
    /// ```
    pub fn check_default<F, Fut>(self, name: impl Into<String>, body: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CheckResult> + Send + 'static,
    {
        self.check(name, Critical::No, DEFAULT_TIMEOUT, body)
    }

    /// Validate the declared checks and freeze them into a [`HealthRegistry`].
    ///
    /// Returns [`BuildError`] if any name is blank, any name is duplicated, or
    /// any timeout is zero.
    pub fn build(self) -> Result<HealthRegistry, BuildError> {
        let mut seen = HashSet::new();
        for check in &self.checks {
            if check.name.trim().is_empty() {
                return Err(BuildError::BlankName);
            }
            if check.timeout.is_zero() {
                return Err(BuildError::NonPositiveTimeout(check.name.clone()));
            }
            if !seen.insert(check.name.as_str()) {
                return Err(BuildError::DuplicateName(check.name.clone()));
            }
        }
        Ok(HealthRegistry {
            checks: self.checks,
        })
    }
}
