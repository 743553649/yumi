package com.yumi.bridge.utils

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test

class SystemStatsParserTest {

    @Test
    fun testParseMemInfo() {
        val fakeMemInfo = """
            MemTotal:       12300000 kB
            MemFree:         2000000 kB
            MemAvailable:    6150000 kB
            Buffers:          200000 kB
            Cached:          1000000 kB
            SwapTotal:       4000000 kB
            SwapFree:        1000000 kB
        """.trimIndent()

        val memStats = SystemStatsParser.parseMemInfo(fakeMemInfo)
        assertNotNull(memStats)

        assertEquals(50, memStats.ramPercent)
        assertEquals("5.9G / 11.7G", memStats.ramDetailText)

        assertEquals(75, memStats.swapPercent)
        assertEquals("2.9G / 3.8G", memStats.swapDetailText)
    }

    @Test
    fun testFormatBatteryPowerMicroAmps() {
        // Raw µA: -2500000 µA * 4.0V = -10.0 W
        val powerWatts = SystemStatsParser.formatPowerWatts(-2500000L, 4000000L)
        assertEquals("-10.0 W", powerWatts)
    }

    @Test
    fun testFormatBatteryPowerMilliAmpsAutoScaling() {
        // Raw mA: -350 mA * 4.0V = -1.4 W (auto scaled from mA to µA)
        val powerWatts = SystemStatsParser.formatPowerWatts(-350L, 4000000L)
        assertEquals("-1.4 W", powerWatts)
    }

    @Test
    fun testFormatTemperature() {
        val tempText = SystemStatsParser.formatTemperature(365)
        assertEquals("36.5 ℃", tempText)
    }

    @Test
    fun testFormatUptimeDaysHoursMins() {
        // 90065 seconds = 1 day, 1 hour, 1 minute, 5 seconds
        val elapsedSec = 90065L
        val days = elapsedSec / 86400
        val hours = (elapsedSec % 86400) / 3600
        val mins = (elapsedSec % 3600) / 60
        val uptimeText = String.format("%d天:%02d小时:%02d分钟", days, hours, mins)
        assertEquals("1天:01小时:01分钟", uptimeText)
    }
}
