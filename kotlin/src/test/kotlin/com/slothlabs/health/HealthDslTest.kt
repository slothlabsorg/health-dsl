package com.slothlabs.health

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.time.Duration.Companion.milliseconds
import kotlinx.coroutines.delay
import kotlinx.coroutines.test.runTest

class HealthDslTest {

    @Test
    fun `all up yields UP`() = runTest {
        val report =
            healthChecks {
                check("a") { up() }
                check("b", critical = true) { up("region" to "us-east-1") }
            }.run()

        assertEquals(Status.UP, report.status)
        assertTrue(report.isHealthy)
        assertEquals(listOf("a", "b"), report.checks.map { it.name })
    }

    @Test
    fun `critical down fails the report`() = runTest {
        val report =
            healthChecks {
                check("db", critical = true) { down("connection refused") }
                check("cache") { up() }
            }.run()

        assertEquals(Status.DOWN, report.status)
        assertFalse(report.isHealthy)
    }

    @Test
    fun `non-critical down only degrades`() = runTest {
        val report =
            healthChecks {
                check("db", critical = true) { up() }
                check("metrics") { down("sink unreachable") }
            }.run()

        assertEquals(Status.DEGRADED, report.status)
        assertTrue(report.isHealthy) // still serving
    }

    @Test
    fun `degraded propagates`() = runTest {
        val report = healthChecks { check("disk") { degraded("low space") } }.run()
        assertEquals(Status.DEGRADED, report.status)
    }

    @Test
    fun `exceptions become DOWN, not thrown`() = runTest {
        val report =
            healthChecks {
                check("boom", critical = true) { error("kaboom") }
            }.run()

        assertEquals(Status.DOWN, report.status)
        assertEquals("kaboom", report.checks.single().message)
    }

    @Test
    fun `timeout becomes DOWN`() = runTest {
        val report =
            healthChecks {
                check("slow", critical = true, timeout = 50.milliseconds) {
                    delay(10_000)
                    up()
                }
            }.run()

        assertEquals(Status.DOWN, report.status)
        assertTrue(report.checks.single().message!!.contains("timed out"))
    }

    @Test
    fun `empty registry is UP`() = runTest {
        val report = healthChecks {}.run()
        assertEquals(Status.UP, report.status)
        assertTrue(report.checks.isEmpty())
    }

    @Test
    fun `duplicate names are rejected`() {
        val ex =
            kotlin.runCatching {
                healthChecks {
                    check("x") { up() }
                    check("x") { up() }
                }
            }.exceptionOrNull()
        assertTrue(ex is IllegalArgumentException)
    }

    @Test
    fun `json renders status, checks and escapes details`() = runTest {
        val report =
            healthChecks {
                check("db", critical = true) { down("line1\n\"quoted\"") }
            }.run()

        val json = report.toJson()
        assertTrue(json.contains("\"status\":\"DOWN\""))
        assertTrue(json.contains("\"db\""))
        assertTrue(json.contains("\"critical\":true"))
        // newline and quote are escaped
        assertTrue(json.contains("line1\\n\\\"quoted\\\""))
    }

    @Test
    fun `details are carried into outcomes`() = runTest {
        val report = healthChecks { check("cache") { up("hitRate" to "0.93") } }.run()
        assertEquals(mapOf("hitRate" to "0.93"), report.checks.single().details)
    }
}
