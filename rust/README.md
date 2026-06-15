# health-dsl

**A tiny async Rust DSL for declaring service readiness/liveness checks — run
concurrently, aggregated correctly, serialized anywhere.**

Most services grow an ad-hoc `/health` endpoint: a pile of `match`/`?` blocks,
inconsistent timeouts, one slow dependency that hangs the whole probe, and no
clear distinction between "we're down" and "we're degraded but still serving."
`health-dsl` makes that a declaration:

```rust
use std::time::Duration;
use health_dsl::{CheckResult, Critical, HealthRegistry};

# async fn run() -> Result<(), health_dsl::BuildError> {
let health = HealthRegistry::builder()
    .check("database", Critical::Yes, Duration::from_secs(5), || async {
        if db_ping().await { CheckResult::up() } else { CheckResult::down("primary unreachable") }
    })
    .check_default("cache", || async {
        CheckResult::up_with([("hitRate", "0.93".to_string())])
    })
    .check("disk", Critical::No, Duration::from_secs(2), || async {
        let free = free_percent().await;
        if free < 5 {
            CheckResult::down(format!("disk almost full: {free}%"))
        } else if free < 15 {
            CheckResult::degraded(format!("disk low: {free}%"))
        } else {
            CheckResult::up_with([("freePercent", free.to_string())])
        }
    })
    .build()?;

let report = health.run().await;  // all checks run concurrently
report.status;                    // Status::Up | Degraded | Down
report.is_healthy();              // false only when Down
report.to_json();
# Ok(())
# }
# async fn db_ping() -> bool { true }
# async fn free_percent() -> u8 { 42 }
```

## What it gives you

- **Correct aggregation.** A `critical` dependency that is `DOWN` makes the
  system `DOWN`; a non-critical failure (or any `degraded`) makes it `DEGRADED`
  but still healthy. You stop conflating "page someone" with "we're fine."
- **Concurrency + per-check timeouts** via `tokio`. `report.duration_ms` is
  roughly your slowest check, not the sum. A hung dependency becomes a `DOWN`
  outcome after its `timeout` instead of hanging the probe.
- **No check can break the report.** Timeouts and panics are folded into a
  `DOWN` outcome, so one misbehaving check cannot abort the whole report.
- **Serialize anywhere.** [`HealthReport`] derives `serde::Serialize`;
  `to_json()` renders a compact document, and `to_value()` hands you a
  `serde_json::Value` to embed wherever you like.

## Install

```sh
cargo add health-dsl
cargo add tokio --features rt-multi-thread,macros,time
```

`health-dsl` uses `tokio` for concurrency and per-check timeouts, and `serde` /
`serde_json` for serialization.

## Status semantics

| Any check…                         | Overall status |
|------------------------------------|----------------|
| critical `DOWN`                    | `DOWN`         |
| non-critical `DOWN`, or `DEGRADED` | `DEGRADED`     |
| all `UP` (or no checks)            | `UP`           |

`Status` is ordered `UP < DEGRADED < DOWN` (the `Ord` derive relies on that
declaration order), so aggregation is just "take the worst," with a non-critical
`DOWN` capped at `DEGRADED`.

## JSON shape

```json
{
  "status": "DEGRADED",
  "durationMs": 41,
  "checks": {
    "database": { "status": "UP", "critical": true, "durationMs": 6 },
    "cache":    { "status": "UP", "critical": false, "durationMs": 2, "details": { "hitRate": "0.93" } },
    "disk":     { "status": "DEGRADED", "critical": false, "durationMs": 3, "message": "disk low: 12%" }
  }
}
```

## Errors as `DOWN`

Rust check bodies do not throw, so the idiomatic path is to map fallible work
to a `DOWN` outcome:

```rust
use health_dsl::CheckResult;

# async fn open_connection() -> Result<(), String> { Ok(()) }
# async fn check_body() -> CheckResult {
match open_connection().await {
    Ok(()) => CheckResult::up(),
    Err(e) => CheckResult::down(e),
}
# }
```

A check that nonetheless **panics** is also caught and folded into `DOWN`, so a
bug in one check never aborts the report.

## License

MIT © SlothLabs
