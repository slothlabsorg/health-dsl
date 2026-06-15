import type { CheckOutcome } from "./check.ts";
import type { Status } from "./status.ts";

/** The serializable shape of a single check within {@link ReportObject}. */
export interface CheckObject {
  status: Status;
  critical: boolean;
  durationMs: number;
  message?: string;
  details?: Record<string, string>;
}

/** The plain-object shape produced by {@link HealthReport.toObject}. */
export interface ReportObject {
  status: Status;
  durationMs: number;
  checks: Record<string, CheckObject>;
}

/**
 * The aggregated result of running a registry's checks.
 *
 * `durationMs` is wall-clock time to run all checks; since they run concurrently
 * this is roughly the slowest check, not their sum.
 */
export class HealthReport {
  /** Overall system status. */
  readonly status: Status;
  /** Per-check outcomes, in declaration order. */
  readonly checks: readonly CheckOutcome[];
  /** Wall-clock time to run all checks. */
  readonly durationMs: number;

  constructor(status: Status, checks: readonly CheckOutcome[], durationMs: number) {
    this.status = status;
    this.checks = checks;
    this.durationMs = durationMs;
  }

  /** True unless the system is `DOWN`; suitable for a readiness gate. */
  get isHealthy(): boolean {
    return this.status !== "DOWN";
  }

  /**
   * A nested plain object, convenient for handing to any serializer or HTTP
   * response without depending on one here. Keys are ordered: outer fields
   * first, then checks in declaration order.
   */
  toObject(): ReportObject {
    const checks: Record<string, CheckObject> = {};
    for (const o of this.checks) {
      const entry: CheckObject = {
        status: o.status,
        critical: o.critical,
        durationMs: o.durationMs,
      };
      if (o.message !== undefined) entry.message = o.message;
      if (o.details !== undefined && Object.keys(o.details).length > 0) {
        entry.details = o.details;
      }
      checks[o.name] = entry;
    }
    return { status: this.status, durationMs: this.durationMs, checks };
  }

  /** Render the report as a JSON document. */
  toJSON(): string {
    return JSON.stringify(this.toObject());
  }
}
