# @slothlabs/health-dsl

**A tiny, zero-dependency TypeScript DSL for declaring service readiness/liveness
checks — run concurrently, aggregated correctly, serialized anywhere.**

Most services grow an ad-hoc `/health` endpoint: a pile of `try/catch` blocks,
inconsistent timeouts, one slow dependency that hangs the whole probe, and no
clear distinction between "we're down" and "we're degraded but still serving."
`health-dsl` makes that a declaration:

```ts
import { healthChecks, up, down, degraded } from "@slothlabs/health-dsl";

const health = healthChecks((c) => {
  c.check("database", { critical: true }, async () =>
    (await db.ping()) ? up() : down("primary unreachable"),
  );
  c.check("cache", async () => up({ hitRate: "0.93" }));
  c.check("disk", { timeoutMs: 2000 }, async () => {
    const free = await freePercent();
    if (free < 5) return down(`disk almost full: ${free}%`);
    if (free < 15) return degraded(`disk low: ${free}%`);
    return up({ freePercent: String(free) });
  });
});

const report = await health.run(); // all checks run concurrently
report.status;    // "UP" | "DEGRADED" | "DOWN"
report.isHealthy; // false only when "DOWN"
report.toJSON();
```

## What it gives you

- **Correct aggregation.** A `critical` dependency that is `DOWN` makes the
  system `DOWN`; a non-critical failure (or any `degraded`) makes it `DEGRADED`
  but still healthy. You stop conflating "page someone" with "we're fine."
- **Concurrency + per-check timeouts** via `Promise.all` / `Promise.race`.
  `report.durationMs` is roughly your slowest check, not the sum. A hung
  dependency becomes a `DOWN` outcome after its `timeoutMs` instead of hanging
  the probe (the timer is always cleared, so there are no leaks).
- **No check can break the report.** Thrown errors and timeouts are folded into a
  `DOWN` outcome; `run()` never rejects because of a failing check.
- **Serialize anywhere.** `toObject()` hands you a plain nested object;
  `toJSON()` returns a JSON string (via `JSON.stringify`, so escaping is handled
  for you). No serializer dependency.

## Install

```sh
npm i @slothlabs/health-dsl
```

Published to npm on each `npm-v*` tag (see
[RELEASING.md](../RELEASING.md)). Zero runtime dependencies. Requires Node 22.6+
(ESM).

## Status semantics

| Any check…                         | Overall status |
|------------------------------------|----------------|
| critical `DOWN`                    | `DOWN`         |
| non-critical `DOWN`, or `DEGRADED` | `DEGRADED`     |
| all `UP` (or no checks)            | `UP`           |

`Status` is ordered `UP < DEGRADED < DOWN`, so aggregation is just "take the
worst," with a non-critical `DOWN` capped at `DEGRADED`.

## The DSL

Declare checks inside the `healthChecks` callback. Each check is an async
function returning a `CheckResult` built with `up()`, `degraded(msg)`, or
`down(msg)` — each accepting an optional `Record<string, string>` of details.

```ts
c.check(name, body);                    // defaults: not critical, 5s timeout
c.check(name, { critical: true }, body);
c.check(name, { timeoutMs: 2000 }, body);
```

- `critical` (default `false`) — a `DOWN` here fails the whole report.
- `timeoutMs` (default `5000`) — exceeding it yields a `DOWN("timed out after …ms")`.

Duplicate names, blank names, and non-positive timeouts are rejected at
registration time.

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

`message` is included only when present; `details` only when non-empty. Checks
appear in declaration order.

## Wiring into an HTTP endpoint

`health-dsl` is framework-agnostic — handing it to any HTTP layer is a few lines:

```ts
server.get("/health", async (_req, res) => {
  const report = await health.run();
  res.status(report.isHealthy ? 200 : 503);
  res.type("application/json").send(report.toJSON());
});
```

## License

MIT © SlothLabs
