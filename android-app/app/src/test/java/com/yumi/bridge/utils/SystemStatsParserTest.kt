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
        // Discharging: -10.0 W
        val dischargingWatts = SystemStatsParser.formatPowerWatts(2500000L, 4000000L, false)
        assertEquals("-10.0 W", dischargingWatts)

        // Charging: +10.0 W
        val chargingWatts = SystemStatsParser.formatPowerWatts(2500000L, 4000000L, true)
        assertEquals("+10.0 W", chargingWatts)
    }

    @Test
    fun testFormatBatteryPowerMilliAmpsAutoScaling() {
        // Discharging with auto-scaling: -1.4 W
        val powerWatts = SystemStatsParser.formatPowerWatts(-350L, 4000000L, false)
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
