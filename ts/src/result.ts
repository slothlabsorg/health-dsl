import type { Status } from "./status.ts";

/** Free-form string-to-string metadata attached to a check outcome. */
export type Details = Record<string, string>;

/**
 * The outcome a check body returns. Construct it with the {@link up},
 * {@link degraded}, and {@link down} helpers rather than directly.
 */
export interface CheckResult {
  readonly status: Status;
  readonly message?: string;
  readonly details?: Details;
}

/**
 * The dependency is fully operational.
 *
 * ```ts
 * c.check("cache", async () => up({ hitRate: "0.93" }));
 * ```
 */
export function up(details?: Details): CheckResult {
  return { status: "UP", details };
}

/** The dependency is impaired but the service can still serve. */
export function degraded(message: string, details?: Details): CheckResult {
  return { status: "DEGRADED", message, details };
}

/** The dependency is unusable. */
export function down(message: string, details?: Details): CheckResult {
  return { status: "DOWN", message, details };
}
