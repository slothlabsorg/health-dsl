package com.slothlabs.health

/**
 * The aggregated result of running a [HealthRegistry].
 *
 * @property status overall system status
 * @property checks per-check outcomes, in declaration order
 * @property durationMs wall-clock time to run all checks (they run concurrently,
 *   so this is roughly the slowest check, not their sum)
 */
data class HealthReport(
    val status: Status,
    val checks: List<CheckOutcome>,
    val durationMs: Long,
) {
    /** True unless the system is [Status.DOWN]; suitable for a readiness gate. */
    val isHealthy: Boolean get() = status != Status.DOWN

    /**
     * A nested map representation, convenient for handing to any serializer
     * (Jackson, kotlinx.serialization, a Spring Actuator `HealthIndicator`, …)
     * without depending on one here.
     */
    fun toMap(): Map<String, Any> =
        linkedMapOf(
            "status" to status.name,
            "durationMs" to durationMs,
            "checks" to
                checks.associate { o ->
                    o.name to
                        buildMap {
                            put("status", o.status.name)
                            put("critical", o.critical)
                            put("durationMs", o.durationMs)
                            o.message?.let { put("message", it) }
                            if (o.details.isNotEmpty()) put("details", o.details)
                        }
                },
        )

    /** Render the report as a compact JSON document, dependency-free. */
    fun toJson(): String = renderJson(toMap())
}

private fun renderJson(value: Any?): String =
    when (value) {
        null -> "null"
        is String -> "\"${escapeJson(value)}\""
        is Boolean, is Int, is Long, is Double -> value.toString()
        is Map<*, *> ->
            value.entries.joinToString(prefix = "{", postfix = "}") { (k, v) ->
                "\"${escapeJson(k.toString())}\":${renderJson(v)}"
            }
        is Iterable<*> -> value.joinToString(prefix = "[", postfix = "]") { renderJson(it) }
        else -> "\"${escapeJson(value.toString())}\""
    }

private fun escapeJson(s: String): String =
    buildString(s.length + 2) {
        for (c in s) {
            when (c) {
                '"' -> append("\\\"")
                '\\' -> append("\\\\")
                '\n' -> append("\\n")
                '\r' -> append("\\r")
                '\t' -> append("\\t")
                else -> if (c < ' ') append("\\u%04x".format(c.code)) else append(c)
            }
        }
    }
