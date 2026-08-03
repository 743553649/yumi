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
    fun testFormatBatteryPowerAndTemp() {
        val powerWatts = SystemStatsParser.formatPowerWatts(-2500000L, 4000000L) // -2.5A * 4.0V = -10.0W
        assertEquals("-10.0 W", powerWatts)

        val tempText = SystemStatsParser.formatTemperature(365)
        assertEquals("36.5 ℃", tempText)
    }
}
