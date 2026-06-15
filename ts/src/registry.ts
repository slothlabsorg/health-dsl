import { type CheckFn, type CheckOutcome, HealthCheck } from "./check.ts";
import { HealthReport } from "./report.ts";
import { type Status, worst } from "./status.ts";

/** Default per-check timeout (ms) when none is given in the DSL. */
export const DEFAULT_TIMEOUT_MS = 5_000;

/** Per-check options accepted by {@link CheckScope.check}. */
export interface CheckOptions {
  /** When true, a `DOWN` result fails the whole report (default `false`). */
  critical?: boolean;
  /** Per-check timeout in milliseconds (default {@link DEFAULT_TIMEOUT_MS}). */
  timeoutMs?: number;
}

/**
 * Aggregate per-check outcomes into an overall {@link Status}:
 *  - a **critical** check that is `DOWN` makes the system `DOWN`;
 *  - a **non-critical** `DOWN`, or any `DEGRADED`, degrades it;
 *  - otherwise the system is `UP` (including when there are no checks).
 */
export function aggregate(outcomes: readonly CheckOutcome[]): Status {
  let overall: Status = "UP";
  for (const o of outcomes) {
    const effective: Status =
      o.status === "DOWN" ? (o.critical ? "DOWN" : "DEGRADED") : o.status;
    overall = worst(overall, effective);
  }
  return overall;
}

/**
 * An immutable set of health checks. Obtain one from {@link healthChecks} and
 * invoke {@link run} to execute every check concurrently and aggregate a
 * {@link HealthReport}.
 */
export class HealthRegistry {
  readonly checks: readonly HealthCheck[];

  constructor(checks: readonly HealthCheck[]) {
    this.checks = checks;
  }

  /** The names of all registered checks, in declaration order. */
  get names(): string[] {
    return this.checks.map((c) => c.name);
  }

  /**
   * Run every check concurrently (each with its own timeout) and aggregate the
   * results. Never rejects for a check failure — failures become `DOWN`
   * outcomes.
   */
  async run(): Promise<HealthReport> {
    const start = performance.now();
    const outcomes = await Promise.all(this.checks.map((c) => c.execute()));
    const durationMs = Math.round(performance.now() - start);
    return new HealthReport(aggregate(outcomes), outcomes, durationMs);
  }
}

/**
 * The scope passed to a {@link healthChecks} builder. Declare checks by calling
 * {@link check}; options are optional and may be omitted entirely.
 */
export class CheckScope {
  private readonly checks: HealthCheck[] = [];
  private readonly seen = new Set<string>();

  /**
   * Declare a check. Options may be omitted:
   *
   * ```ts
   * c.check("cache", async () => up());
   * c.check("database", { critical: true }, async () => up());
   * c.check("disk", { timeoutMs: 2000 }, async () => degraded("low"));
   * ```
   */
  check(name: string, body: CheckFn): void;
  check(name: string, options: CheckOptions, body: CheckFn): void;
  check(name: string, optionsOrBody: CheckOptions | CheckFn, maybeBody?: CheckFn): void {
    const options = typeof optionsOrBody === "function" ? {} : optionsOrBody;
    const body = typeof optionsOrBody === "function" ? optionsOrBody : maybeBody;

    if (body === undefined) {
      throw new Error(`check '${name}' is missing a body`);
    }
    if (name.trim().length === 0) {
      throw new Error("check name must not be blank");
    }
    if (this.seen.has(name)) {
      throw new Error(`duplicate check name: ${name}`);
    }
    const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    if (!(timeoutMs > 0)) {
      throw new Error(`timeout for '${name}' must be positive`);
    }

    this.seen.add(name);
    this.checks.push(new HealthCheck(name, options.critical ?? false, timeoutMs, body));
  }

  /** @internal Finalize the declared checks into an immutable registry. */
  build(): HealthRegistry {
    return new HealthRegistry([...this.checks]);
  }
}

/**
 * Entry point to the DSL.
 *
 * ```ts
 * const health = healthChecks((c) => {
 *   c.check("database", { critical: true }, async () =>
 *     (await db.ping()) ? up() : down("unreachable"),
 *   );
 *   c.check("cache", async () => up({ hitRate: "0.93" }));
 * });
 * const report = await health.run();
 * ```
 */
export function healthChecks(build: (c: CheckScope) => void): HealthRegistry {
  const scope = new CheckScope();
  build(scope);
  return scope.build();
}
