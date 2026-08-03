package com.yumi.bridge.utils

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Unit tests for CpuStatsParser logic.
 */
class CpuStatsParserTest {

    @Test
    fun testParseCpuFreqKHzToMHz() {
        val kHzValue = 1804800L
        val mHzValue = CpuStatsParser.khzToMhz(kHzValue)
        assertEquals(1804L, mHzValue)
    }

    @Test
    fun testParseFreqLine() {
        val line = " 800000 "
        val mHzValue = CpuStatsParser.parseFreqLineToMhz(line)
        assertEquals(800L, mHzValue)
    }

    @Test
    fun testParseCpuStatLineAndUsage() {
        // cpu0 user nice system idle iowait irq softirq steal
        val statLine1 = "cpu0 100 0 100 800 0 0 0 0" // total=1000, idle=800
        val statLine2 = "cpu0 200 0 200 1200 0 0 0 0" // total=1600, idle=1200
        // diffTotal = 600, diffIdle = 400, busy = 200 => usage = 200 * 100 / 600 = 33%

        val snapshot1 = CpuStatsParser.parseStatLine(statLine1)
        val snapshot2 = CpuStatsParser.parseStatLine(statLine2)

        assertEquals(0, snapshot1?.cpuId)
        assertEquals(1000L, snapshot1?.totalTime)
        assertEquals(800L, snapshot1?.idleTime)

        val usage = CpuStatsParser.calculateUsage(snapshot1!!, snapshot2!!)
        assertEquals(33, usage)
    }
}
