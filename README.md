# health-dsl

**A tiny DSL for declaring service readiness/liveness checks — run concurrently,
aggregated correctly, serialized anywhere. Three native implementations.**

Most services grow an ad-hoc `/health` endpoint: a pile of `try/catch` blocks,
inconsistent timeouts, one slow dependency that hangs the whole probe, and no
clear distinction between "we're down" and "we're degraded but still serving."
`health-dsl` makes that a declaration.

```kotlin
// Kotlin
val health = healthChecks {
    check("database", critical = true) { if (db.ping()) up() else down("unreachable") }
    check("cache") { up("hitRate" to "0.93") }
    check("disk", timeout = 2.seconds) {
        val free = freePercent()
        when {
            free < 5  -> down("disk almost full: $free%")
            free < 15 -> degraded("disk low: $free%")
            else      -> up("freePercent" to free.toString())
        }
    }
}
val report = health.run()      // all checks run concurrently
report.status                  // UP | DEGRADED | DOWN
```

## What it gives you

- **Correct aggregation.** A `critical` dependency that is `DOWN` makes the
  system `DOWN`; a non-critical failure (or any `degraded`) makes it `DEGRADED`
  but still healthy. You stop conflating "page someone" with "we're fine."
- **Concurrency + per-check timeouts.** `report.durationMs` is roughly your
  slowest check, not the sum. A hung dependency becomes a `DOWN` outcome after
  its timeout instead of hanging the probe.
- **No check can break the report.** Exceptions/panics and timeouts fold into a
  `DOWN` outcome (cancellation is still propagated).
- **Serialize anywhere.** A dependency-free JSON renderer, plus a map/struct form
  for Jackson, kotlinx.serialization, serde, or a Spring Actuator
  `HealthIndicator`.

## Implementations

| Language   | Path                 | Install                          | Async model               |
|------------|----------------------|----------------------------------|---------------------------|
| Rust       | [`rust/`](rust/)     | `cargo add health-dsl`           | `tokio` + `join_all`      |
| TypeScript | [`ts/`](ts/)         | `npm i @slothlabs/health-dsl`    | `Promise.all` (zero deps) |
| Kotlin/JVM | [`kotlin/`](kotlin/) | `com.slothlabs:health-dsl`       | coroutines                |

## Install

Each implementation lives in its own subdirectory of this one repository and is
published to its language's registry. You can install from a registry release,
or pull straight from git/JitPack today.

### Rust

From git (works immediately, no registry release needed):

```sh
cargo add health-dsl --git https://github.com/slothlabsorg/health-dsl
```

Or, once a `rust-v*` tag is published to crates.io:

```sh
cargo add health-dsl
cargo add tokio --features rt-multi-thread,macros,time
```

### Kotlin / JVM (JitPack — works immediately)

JitPack builds the `kotlin/` module straight from a git tag — no manual publish
step required.

```kotlin
// build.gradle.kts
repositories {
    mavenCentral()
    maven("https://jitpack.io")
}

dependencies {
    implementation("com.github.slothlabsorg:health-dsl:0.1.0")
}
```

The `0.1.0` coordinate resolves to the git tag `v0.1.0`. (Releases pushed to
GitHub Packages under `com.slothlabs:health-dsl` via a `jvm-v*` tag are an
alternative; see [RELEASING.md](RELEASING.md).)

### TypeScript

```sh
npm i @slothlabs/health-dsl
```

Available on npm once the `npm-v*` tag publishes the package (see
[RELEASING.md](RELEASING.md)).

## Status semantics (identical across languages)

| Any check…                         | Overall status |
|------------------------------------|----------------|
| critical `DOWN`                    | `DOWN`         |
| non-critical `DOWN`, or `DEGRADED` | `DEGRADED`     |
| all `UP` (or no checks)            | `UP`           |

`Status` is ordered `UP < DEGRADED < DOWN`, so aggregation is "take the worst,"
with a non-critical `DOWN` capped at `DEGRADED`.

See each subdirectory's README for the language-specific API.

## License

MIT © SlothLabs
