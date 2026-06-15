package com.slothlabs.health

/**
 * The health of a single check or of the system as a whole.
 *
 * Ordered from healthiest to least healthy so that aggregation can simply take
 * the "worst" of a set: [UP] < [DEGRADED] < [DOWN].
 */
enum class Status {
    /** Fully operational. */
    UP,

    /** Serving, but impaired — a non-critical dependency is unhealthy. */
    DEGRADED,

    /** Not serving — a critical dependency is unhealthy. */
    DOWN,
}

/** Return the less-healthy of two statuses. */
internal fun worst(a: Status, b: Status): Status = if (a.ordinal >= b.ordinal) a else b
