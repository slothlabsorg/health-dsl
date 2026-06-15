import type { CheckResult, Details } from "./result.ts";
import type { Status } from "./status.ts";

/**
 * A check body: an async function returning a {@link CheckResult}. Use the
 * {@link up}, {@link degraded}, and {@link down} helpers to build the result.
 */
export type CheckFn = () => CheckResult | Promise<CheckResult>;

/** The recorded result of running a single {@link HealthCheck}. */
export interface CheckOutcome {
  readonly name: string;
  readonly critical: boolean;
  readonly status: Status;
  readonly message?: string;
  readonly details?: Details;
  readonly durationMs: number;
}

/**
 * A single named health check. Created via the {@link healthChecks} DSL, not
 * directly.
 *
 * - `critical`: when true, a `DOWN` result fails the whole report; when false it
 *   only degrades it.
 * - `timeoutMs`: the check is considered `DOWN` if it does not finish within this
 *   many milliseconds.
 */
export class HealthCheck {
  readonly name: string;
  readonly critical: boolean;
  readonly timeoutMs: number;
  private readonly body: CheckFn;

  constructor(name: string, critical: boolean, timeoutMs: number, body: CheckFn) {
    this.name = name;
    this.critical = critical;
    this.timeoutMs = timeoutMs;
    this.body = body;
  }

  /**
   * Run the check, never rejecting: timeouts and thrown errors are folded into a
   * `DOWN` outcome so one misbehaving check cannot break the report.
   */
  async execute(): Promise<CheckOutcome> {
    const start = performance.now();
    const result = await this.runGuarded();
    const durationMs = Math.round(performance.now() - start);
    return {
      name: this.name,
      critical: this.critical,
      status: result.status,
      message: result.message,
      details: result.details,
      durationMs,
    };
  }

  private async runGuarded(): Promise<CheckResult> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const timeout = new Promise<CheckResult>((resolve) => {
      timer = setTimeout(
        () => resolve({ status: "DOWN", message: `timed out after ${this.timeoutMs}ms` }),
        this.timeoutMs,
      );
    });
    try {
      // Wrap the body call so a synchronous throw is also captured by the race.
      return await Promise.race([Promise.resolve().then(this.body), timeout]);
    } catch (err) {
      return { status: "DOWN", message: errorMessage(err) };
    } finally {
      if (timer !== undefined) clearTimeout(timer);
    }
  }
}

/** Best-effort human-readable message for an arbitrary thrown value. */
function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message || err.name || "error";
  if (typeof err === "string" && err.length > 0) return err;
  return "error";
}
