package com.yumi.bridge.ui.compose

import com.yumi.bridge.MainActivity
import org.junit.Assert.assertEquals
import org.junit.Test

class LogFilterTest {

    @Test
    fun filterLogs_levelAll_returnsAllLogs() {
        val logs = listOf(
            MainActivity.RealLogEntry("log1", "log1", MainActivity.LEVEL_INFO),
            MainActivity.RealLogEntry("log2", "log2", MainActivity.LEVEL_ERROR),
            MainActivity.RealLogEntry("log3", "log3", MainActivity.LEVEL_DEBUG)
        )
        val filtered = filterLogs(logs, MainActivity.LEVEL_ALL)
        assertEquals(3, filtered.size)
    }

    @Test
    fun filterLogs_specificLevel_returnsMatchingLogsOnly() {
        val logs = listOf(
            MainActivity.RealLogEntry("log1", "log1", MainActivity.LEVEL_INFO),
            MainActivity.RealLogEntry("log2", "log2", MainActivity.LEVEL_ERROR),
            MainActivity.RealLogEntry("log3", "log3", MainActivity.LEVEL_DEBUG)
        )
        val filteredInfo = filterLogs(logs, MainActivity.LEVEL_INFO)
        assertEquals(1, filteredInfo.size)
        assertEquals(MainActivity.LEVEL_INFO, filteredInfo[0].level)

        val filteredError = filterLogs(logs, MainActivity.LEVEL_ERROR)
        assertEquals(1, filteredError.size)
        assertEquals(MainActivity.LEVEL_ERROR, filteredError[0].level)
    }
}
