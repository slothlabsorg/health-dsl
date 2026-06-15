package com.slothlabs.health

/**
 * The outcome a check body returns. Construct it with the [CheckScope] helpers
 * ([CheckScope.up], [CheckScope.degraded], [CheckScope.down]) rather than
 * directly.
 */
data class CheckResult(
    val status: Status,
    val message: String? = null,
    val details: Map<String, String> = emptyMap(),
)

/**
 * Receiver for a check body. Gives each check terse, intention-revealing
 * constructors so the body reads as a decision table:
 *
 * ```kotlin
 * check("disk") {
 *     val free = freePercent()
 *     when {
 *         free < 5  -> down("disk almost full: $free%")
 *         free < 15 -> degraded("disk low: $free%")
 *         else      -> up("freePercent" to free.toString())
 *     }
 * }
 * ```
 */
object CheckScope {
    /** The dependency is fully operational. */
    fun up(details: Map<String, String> = emptyMap()): CheckResult =
        CheckResult(Status.UP, null, details)

    /** Convenience for `up(mapOf(pairs))`. */
    fun up(vararg details: Pair<String, String>): CheckResult =
        CheckResult(Status.UP, null, details.toMap())

    /** The dependency is impaired but the service can still serve. */
    fun degraded(message: String, details: Map<String, String> = emptyMap()): CheckResult =
        CheckResult(Status.DEGRADED, message, details)

    /** The dependency is unusable. */
    fun down(message: String, details: Map<String, String> = emptyMap()): CheckResult =
        CheckResult(Status.DOWN, message, details)
}
