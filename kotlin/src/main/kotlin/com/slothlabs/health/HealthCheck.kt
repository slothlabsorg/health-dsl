package com.slothlabs.health

import kotlin.time.Duration
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.withTimeoutOrNull

/**
 * A single named health check. Created via the [healthChecks] DSL, not directly.
 *
 * @property name unique, human-readable identifier
 * @property critical when true, a [Status.DOWN] result fails the whole report;
 *   when false it only degrades it
 * @property timeout the check is considered [Status.DOWN] if it does not finish
 *   within this duration
 */
class HealthCheck internal constructor(
    val name: String,
    val critical: Boolean,
    val timeout: Duration,
    private val block: suspend CheckScope.() -> CheckResult,
) {
    /**
     * Run the check, never throwing: timeouts and exceptions are folded into a
     * [Status.DOWN] outcome so one misbehaving check cannot break the report.
     * Structured-concurrency cancellation is propagated.
     */
    suspend fun execute(): CheckOutcome {
        val start = System.nanoTime()
        val result =
            try {
                withTimeoutOrNull(timeout) { CheckScope.block() }
                    ?: CheckResult(Status.DOWN, "timed out after $timeout")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Throwable) {
                CheckResult(Status.DOWN, e.message ?: e::class.simpleName ?: "error")
            }
        val durationMs = (System.nanoTime() - start) / 1_000_000
        return CheckOutcome(
            name = name,
            critical = critical,
            status = result.status,
            message = result.message,
            details = result.details,
            durationMs = durationMs,
        )
    }
}

/** The recorded result of running a [HealthCheck]. */
data class CheckOutcome(
    val name: String,
    val critical: Boolean,
    val status: Status,
    val message: String?,
    val details: Map<String, String>,
    val durationMs: Long,
)
