//! Integration tests mirroring the reference Kotlin `HealthDslTest`.

use std::time::Duration;

use health_dsl::{BuildError, CheckResult, Critical, HealthRegistry, Status};

#[tokio::test]
async fn all_up_yields_up() {
    let report = HealthRegistry::builder()
        .check_default("a", || async { CheckResult::up() })
        .check("b", Critical::Yes, Duration::from_secs(5), || async {
            CheckResult::up_with([("region", "us-east-1".to_string())])
        })
        .build()
        .unwrap()
        .run()
        .await;

    assert_eq!(report.status, Status::Up);
    assert!(report.is_healthy());
    let names: Vec<&str> = report.checks.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b"]);
}

#[tokio::test]
async fn critical_down_fails_the_report() {
    let report = HealthRegistry::builder()
        .check("db", Critical::Yes, Duration::from_secs(5), || async {
            CheckResult::down("connection refused")
        })
        .check_default("cache", || async { CheckResult::up() })
        .build()
        .unwrap()
        .run()
        .await;

    assert_eq!(report.status, Status::Down);
    assert!(!report.is_healthy());
}

#[tokio::test]
async fn non_critical_down_only_degrades() {
    let report = HealthRegistry::builder()
        .check("db", Critical::Yes, Duration::from_secs(5), || async {
            CheckResult::up()
        })
        .check_default("metrics", || async {
            CheckResult::down("sink unreachable")
        })
        .build()
        .unwrap()
        .run()
        .await;

    assert_eq!(report.status, Status::Degraded);
    assert!(report.is_healthy()); // still serving
}

#[tokio::test]
async fn degraded_propagates() {
    let report = HealthRegistry::builder()
        .check_default("disk", || async { CheckResult::degraded("low space") })
        .build()
        .unwrap()
        .run()
        .await;

    assert_eq!(report.status, Status::Degraded);
}

#[tokio::test]
async fn panics_become_down_not_thrown() {
    let report = HealthRegistry::builder()
        .check("boom", Critical::Yes, Duration::from_secs(5), || async {
            panic!("kaboom");
        })
        .build()
        .unwrap()
        .run()
        .await;

    assert_eq!(report.status, Status::Down);
    assert_eq!(report.checks[0].message.as_deref(), Some("kaboom"));
}

#[tokio::test]
async fn errors_become_down_via_returning() {
    // The idiomatic path: wrap fallible work in a Result and map Err -> down.
    fn ping() -> Result<(), String> {
        Err("connection refused".to_string())
    }
    let report = HealthRegistry::builder()
        .check("db", Critical::Yes, Duration::from_secs(5), || async {
            match ping() {
                Ok(()) => CheckResult::up(),
                Err(e) => CheckResult::down(e),
            }
        })
        .build()
        .unwrap()
        .run()
        .await;

    assert_eq!(report.status, Status::Down);
    assert_eq!(
        report.checks[0].message.as_deref(),
        Some("connection refused")
    );
}

#[tokio::test]
async fn timeout_becomes_down() {
    let report = HealthRegistry::builder()
        .check("slow", Critical::Yes, Duration::from_millis(50), || async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            CheckResult::up()
        })
        .build()
        .unwrap()
        .run()
        .await;

    assert_eq!(report.status, Status::Down);
    assert!(report.checks[0]
        .message
        .as_deref()
        .unwrap()
        .contains("timed out"));
}

#[tokio::test]
async fn empty_registry_is_up() {
    let report = HealthRegistry::builder().build().unwrap().run().await;
    assert_eq!(report.status, Status::Up);
    assert!(report.checks.is_empty());
}

#[test]
fn duplicate_names_are_rejected() {
    let err = HealthRegistry::builder()
        .check_default("x", || async { CheckResult::up() })
        .check_default("x", || async { CheckResult::up() })
        .build()
        .unwrap_err();
    assert_eq!(err, BuildError::DuplicateName("x".to_string()));
}

#[test]
fn blank_names_are_rejected() {
    let err = HealthRegistry::builder()
        .check_default("  ", || async { CheckResult::up() })
        .build()
        .unwrap_err();
    assert_eq!(err, BuildError::BlankName);
}

#[tokio::test]
async fn json_renders_status_checks_and_escapes_message() {
    let report = HealthRegistry::builder()
        .check("db", Critical::Yes, Duration::from_secs(5), || async {
            CheckResult::down("line1\n\"quoted\"")
        })
        .build()
        .unwrap()
        .run()
        .await;

    let json = report.to_json();
    assert!(json.contains("\"status\":\"DOWN\""));
    assert!(json.contains("\"db\""));
    assert!(json.contains("\"critical\":true"));
    // newline and quote are escaped
    assert!(json.contains("line1\\n\\\"quoted\\\""));
}

#[tokio::test]
async fn details_are_carried_into_outcomes() {
    let report = HealthRegistry::builder()
        .check_default("cache", || async {
            CheckResult::up_with([("hitRate", "0.93".to_string())])
        })
        .build()
        .unwrap()
        .run()
        .await;

    let outcome = &report.checks[0];
    assert_eq!(
        outcome.details.get("hitRate").map(String::as_str),
        Some("0.93")
    );
}
