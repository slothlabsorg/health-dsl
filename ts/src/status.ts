/**
 * The health of a single check or of the system as a whole.
 *
 * Ordered from healthiest to least healthy so that aggregation can simply take
 * the "worst" of a set: `UP` < `DEGRADED` < `DOWN`.
 */
export type Status = "UP" | "DEGRADED" | "DOWN";

/** Severity rank for each {@link Status}; higher means less healthy. */
const RANK: Record<Status, number> = {
  UP: 0,
  DEGRADED: 1,
  DOWN: 2,
};

/** Return the less-healthy of two statuses. */
export function worst(a: Status, b: Status): Status {
  return RANK[a] >= RANK[b] ? a : b;
}
