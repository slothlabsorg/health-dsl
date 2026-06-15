# health-dsl

**A tiny Kotlin DSL for declaring service readiness/liveness checks — run
concurrently, aggregated correctly, serialized anywhere.**

Most services grow an ad-hoc `/health` endpoint: a pile of `try/catch` blocks,
inconsistent timeouts, one slow dependency that hangs the whole probe, and no
clear distinction between "we're down" and "we're degraded but still serving."
`health-dsl` makes that a declaration:

```kotlin
import com.slothlabs.health.*

val health = healthChecks {
    check("database", critical = true) {
        if (db.ping()) up() else down("primary unreachable")
    }
    check("cache") {
        up("hitRate" to "0.93")
    }
    check("disk", timeout = 2.seconds) {
        val free = freePercent()
        when {
            free < 5  -> down("disk almost full: $free%")
            free < 15 -> degraded("disk low: $free%")
            else      -> up("freePercent" to free.toString())
        }
    }
}

val report = health.run()      // suspend; all checks run concurrently
report.status                  // UP | DEGRADED | DOWN
report.isHealthy               // false only when DOWN
report.toJson()
```

## What it gives you

- **Correct aggregation.** A `critical` dependency that is `DOWN` makes the
  system `DOWN`; a non-critical failure (or any `degraded`) makes it `DEGRADED`
  but still healthy. You stop conflating "page someone" with "we're fine."
- **Concurrency + per-check timeouts** via Kotlin coroutines. `report.durationMs`
  is roughly your slowest check, not the sum. A hung dependency becomes a `DOWN`
  outcome after its `timeout` instead of hanging the probe.
- **No check can break the report.** Exceptions and timeouts are folded into a
  `DOWN` outcome (structured-concurrency cancellation is still propagated).
- **Serialize anywhere.** `toJson()` is dependency-free; `toMap()` hands a nested
  map to Jackson, kotlinx.serialization, or a Spring Actuator `HealthIndicator`.

## Install

```kotlin
// build.gradle.kts
dependencies {
    implementation("com.slothlabs:health-dsl:0.1.0")
}
```

Requires Kotlin 2.x and `kotlinx-coroutines` (declared transitively).

## Status semantics

| Any check…                         | Overall status |
|------------------------------------|----------------|
| critical `DOWN`                    | `DOWN`         |
| non-critical `DOWN`, or `DEGRADED` | `DEGRADED`     |
| all `UP` (or no checks)            | `UP`           |

`Status` is ordered `UP < DEGRADED < DOWN`, so aggregation is just "take the
worst," with a non-critical `DOWN` capped at `DEGRADED`.

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

## Spring Boot Actuator

`health-dsl` is framework-agnostic; bridging to Actuator is a few lines and
needs no dependency from this library:

```kotlin
@Component
class DependencyHealthIndicator(private val health: HealthRegistry) : HealthIndicator {
    override fun health(): org.springframework.boot.actuate.health.Health {
        val report = runBlocking { health.run() }
        val builder = when (report.status) {
            Status.UP       -> org.springframework.boot.actuate.health.Health.up()
            Status.DEGRADED -> org.springframework.boot.actuate.health.Health.status("DEGRADED")
            Status.DOWN     -> org.springframework.boot.actuate.health.Health.down()
        }
        return builder.withDetails(report.toMap()).build()
    }
}
```

## License

MIT © SlothLabs
