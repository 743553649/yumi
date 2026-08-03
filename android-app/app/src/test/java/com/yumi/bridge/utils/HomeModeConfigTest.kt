package com.yumi.bridge.utils

import com.yumi.bridge.ui.compose.SUPPORTED_MODES
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class HomeModeConfigTest {

    @Test
    fun testSupportedModesCountAndExclusion() {
        // Must contain exactly 4 modes: powersave, balance, performance, fast
        assertEquals(4, SUPPORTED_MODES.size)
        val modeKeys = SUPPORTED_MODES.map { it.key }
        assertFalse(modeKeys.contains("fas"))
    }
}
