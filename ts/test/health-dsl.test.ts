import assert from "node:assert/strict";
import { test } from "node:test";

import { degraded, down, healthChecks, up } from "../src/index.ts";

const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

test("all up yields UP", async () => {
  const report = await healthChecks((c) => {
    c.check("a", async () => up());
    c.check("b", { critical: true }, async () => up({ region: "us-east-1" }));
  }).run();

  assert.equal(report.status, "UP");
  assert.equal(report.isHealthy, true);
  assert.deepEqual(
    report.checks.map((o) => o.name),
    ["a", "b"],
  );
});

test("critical down fails the report", async () => {
  const report = await healthChecks((c) => {
    c.check("db", { critical: true }, async () => down("connection refused"));
    c.check("cache", async () => up());
  }).run();

  assert.equal(report.status, "DOWN");
  assert.equal(report.isHealthy, false);
});

test("non-critical down only degrades (still healthy)", async () => {
  const report = await healthChecks((c) => {
    c.check("db", { critical: true }, async () => up());
    c.check("metrics", async () => down("sink unreachable"));
  }).run();

  assert.equal(report.status, "DEGRADED");
  assert.equal(report.isHealthy, true);
});

test("degraded propagates", async () => {
  const report = await healthChecks((c) => {
    c.check("disk", async () => degraded("low space"));
  }).run();

  assert.equal(report.status, "DEGRADED");
});

test("exceptions become DOWN, not rejected", async () => {
  const report = await healthChecks((c) => {
    c.check("boom", { critical: true }, async () => {
      throw new Error("kaboom");
    });
  }).run();

  assert.equal(report.status, "DOWN");
  assert.equal(report.checks.length, 1);
  assert.equal(report.checks[0]?.message, "kaboom");
});

test("synchronous throw in a check also becomes DOWN", async () => {
  const report = await healthChecks((c) => {
    // Not async; throws synchronously.
    c.check("sync-boom", { critical: true }, (): never => {
      throw new Error("sync kaboom");
    });
  }).run();

  assert.equal(report.status, "DOWN");
  assert.equal(report.checks[0]?.message, "sync kaboom");
});

test("timeout becomes DOWN with a 'timed out' message", async () => {
  const report = await healthChecks((c) => {
    c.check("slow", { critical: true, timeoutMs: 50 }, async () => {
      await delay(10_000);
      return up();
    });
  }).run();

  assert.equal(report.status, "DOWN");
  assert.match(report.checks[0]?.message ?? "", /timed out/);
});

test("empty registry is UP", async () => {
  const report = await healthChecks(() => {}).run();
  assert.equal(report.status, "UP");
  assert.equal(report.checks.length, 0);
  assert.equal(report.isHealthy, true);
});

test("duplicate names are rejected at registration", () => {
  assert.throws(
    () =>
      healthChecks((c) => {
        c.check("x", async () => up());
        c.check("x", async () => up());
      }),
    /duplicate check name: x/,
  );
});

test("blank names are rejected", () => {
  assert.throws(
    () => healthChecks((c) => c.check("   ", async () => up())),
    /must not be blank/,
  );
});

test("non-positive timeout is rejected", () => {
  assert.throws(
    () => healthChecks((c) => c.check("z", { timeoutMs: 0 }, async () => up())),
    /must be positive/,
  );
});

test("toObject has the correct nested shape incl. details and message", async () => {
  const report = await healthChecks((c) => {
    c.check("database", { critical: true }, async () => up());
    c.check("cache", async () => up({ hitRate: "0.93" }));
    c.check("disk", async () => degraded("disk low: 12%"));
  }).run();

  const obj = report.toObject();
  assert.equal(obj.status, "DEGRADED");
  assert.equal(typeof obj.durationMs, "number");

  // Declaration order preserved in the checks object.
  assert.deepEqual(Object.keys(obj.checks), ["database", "cache", "disk"]);

  assert.deepEqual(obj.checks["database"], {
    status: "UP",
    critical: true,
    durationMs: obj.checks["database"]?.durationMs ?? 0,
  });
  // No message / no details when absent.
  assert.equal("message" in (obj.checks["database"] ?? {}), false);
  assert.equal("details" in (obj.checks["database"] ?? {}), false);

  assert.deepEqual(obj.checks["cache"]?.details, { hitRate: "0.93" });
  assert.equal(obj.checks["disk"]?.message, "disk low: 12%");
});

test("toJSON renders status, checks, and escapes special characters", async () => {
  const report = await healthChecks((c) => {
    c.check("db", { critical: true }, async () => down('line1\n"quoted"'));
  }).run();

  const json = report.toJSON();
  assert.match(json, /"status":"DOWN"/);
  assert.match(json, /"db"/);
  assert.match(json, /"critical":true/);
  // JSON.stringify escapes newline and quote.
  assert.ok(json.includes('line1\\n\\"quoted\\"'));

  // Round-trips back to the same object.
  assert.deepEqual(JSON.parse(json), report.toObject());
});

test("details are carried into outcomes", async () => {
  const report = await healthChecks((c) => {
    c.check("cache", async () => up({ hitRate: "0.93" }));
  }).run();

  assert.deepEqual(report.checks[0]?.details, { hitRate: "0.93" });
});

test("outcomes preserve declaration order even with varied durations", async () => {
  const report = await healthChecks((c) => {
    c.check("first", async () => {
      await delay(30);
      return up();
    });
    c.check("second", async () => up());
    c.check("third", async () => {
      await delay(10);
      return up();
    });
  }).run();

  assert.deepEqual(
    report.checks.map((o) => o.name),
    ["first", "second", "third"],
  );
});

test("checks run concurrently, not serially", async () => {
  const start = performance.now();
  await healthChecks((c) => {
    c.check("a", async () => {
      await delay(80);
      return up();
    });
    c.check("b", async () => {
      await delay(80);
      return up();
    });
  }).run();
  const elapsed = performance.now() - start;
  // Serial would be ~160ms; concurrent should be well under.
  assert.ok(elapsed < 150, `expected concurrent run, took ${elapsed}ms`);
});
