/**
 * # @slothlabs/health-dsl
 *
 * A tiny, zero-dependency DSL for declaring service readiness/liveness checks —
 * run concurrently with per-check timeouts, aggregated correctly into an overall
 * status, and serialized to JSON.
 *
 * ```ts
 * import { healthChecks, up, down, degraded } from "@slothlabs/health-dsl";
 *
 * const health = healthChecks((c) => {
 *   c.check("database", { critical: true }, async () =>
 *     (await db.ping()) ? up() : down("primary unreachable"),
 *   );
 *   c.check("cache", async () => up({ hitRate: "0.93" }));
 *   c.check("disk", { timeoutMs: 2000 }, async () => {
 *     const free = await freePercent();
 *     if (free < 5) return down(`disk almost full: ${free}%`);
 *     if (free < 15) return degraded(`disk low: ${free}%`);
 *     return up({ freePercent: String(free) });
 *   });
 * });
 *
 * const report = await health.run(); // all checks run concurrently
 * report.status;    // "UP" | "DEGRADED" | "DOWN"
 * report.isHealthy; // false only when "DOWN"
 * report.toJSON();
 * ```
 */

export { up, degraded, down } from "./result.ts";
export type { CheckResult, Details } from "./result.ts";

export type { Status } from "./status.ts";
export { worst } from "./status.ts";

export { HealthCheck } from "./check.ts";
export type { CheckFn, CheckOutcome } from "./check.ts";

export { HealthReport } from "./report.ts";
export type { CheckObject, ReportObject } from "./report.ts";

export {
  healthChecks,
  HealthRegistry,
  CheckScope,
  aggregate,
  DEFAULT_TIMEOUT_MS,
} from "./registry.ts";
export type { CheckOptions } from "./registry.ts";
