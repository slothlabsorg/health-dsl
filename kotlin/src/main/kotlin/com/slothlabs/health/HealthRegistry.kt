package com.slothlabs.health

import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope

/** Default per-check timeout when none is given in the DSL. */
val DEFAULT_TIMEOUT: Duration = 5.seconds

/**
 * An immutable set of health checks. Obtain one from [healthChecks] and invoke
 * [run] to execute every check concurrently and aggregate a [HealthReport].
 */
class HealthRegistry internal constructor(val checks: List<HealthCheck>) {

    /** The names of all registered checks, in declaration order. */
    val names: List<String> get() = checks.map { it.name }

    /**
     * Run every check concurrently (each with its own timeout) and aggregate
     * the results. Suspends until all checks complete. Never throws for a check
     * failure — failures become [Status.DOWN] outcomes.
     */
    suspend fun run(): HealthReport = coroutineScope {
        val start = System.nanoTime()
        val outcomes = checks.map { check -> async { check.execute() } }.awaitAll()
        val durationMs = (System.nanoTime() - start) / 1_000_000
        HealthReport(aggregate(outcomes), outcomes, durationMs)
    }
}

/**
 * Aggregate per-check outcomes into an overall [Status]:
 *  - a **critical** check that is [Status.DOWN] makes the system [Status.DOWN];
 *  - a **non-critical** [Status.DOWN], or any [Status.DEGRADED], degrades it;
 *  - otherwise the system is [Status.UP] (including when there are no checks).
 */
internal fun aggregate(outcomes: List<CheckOutcome>): Status {
    var overall = Status.UP
    for (o in outcomes) {
        val effective =
            when (o.status) {
                Status.DOWN -> if (o.critical) Status.DOWN else Status.DEGRADED
                Status.DEGRADED -> Status.DEGRADED
                Status.UP -> Status.UP
            }
        overall = worst(overall, effective)
    }
    return overall
}

/** Builder backing the [healthChecks] DSL. */
class HealthRegistryBuilder internal constructor() {
    private val checks = mutableListOf<HealthCheck>()

    /**
     * Declare a check.
     *
     * @param name unique identifier
     * @param critical whether a failure should fail the whole report (default false)
     * @param timeout per-check timeout (default [DEFAULT_TIMEOUT])
     * @param block the check body, run on a [CheckScope] receiver
     */
    fun check(
        name: String,
        critical: Boolean = false,
        timeout: Duration = DEFAULT_TIMEOUT,
        block: suspend CheckScope.() -> CheckResult,
    ) {
        require(name.isNotBlank()) { "check name must not be blank" }
        require(checks.none { it.name == name }) { "duplicate check name: $name" }
        require(timeout.isPositive()) { "timeout for '$name' must be positive" }
        checks += HealthCheck(name, critical, timeout, block)
    }

    internal fun build(): HealthRegistry = HealthRegistry(checks.toList())
}

/**
 * Entry point to the DSL.
 *
 * ```kotlin
 * val health = healthChecks {
 *     check("database", critical = true) { if (db.ping()) up() else down("unreachable") }
 *     check("cache") { up("hitRate" to "0.93") }
 * }
 * val report = health.run()
 * ```
 */
fun healthChecks(block: HealthRegistryBuilder.() -> Unit): HealthRegistry =
    HealthRegistryBuilder().apply(block).build()
