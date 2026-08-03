package com.yumi.bridge.ui.compose

import org.junit.Assert.assertEquals
import org.junit.Test

class MainNavigationTest {

    @Test
    fun homeUiState_tabSwitching_updatesActiveTabCorrectly() {
        val state = HomeUiState()
        assertEquals(0, state.activeTab)

        var selectedTab = -1
        val onTabSelected: (Int) -> Unit = { index ->
            state.activeTab = index
            selectedTab = index
        }

        onTabSelected(1)
        assertEquals(1, state.activeTab)
        assertEquals(1, selectedTab)

        onTabSelected(2)
        assertEquals(2, state.activeTab)
        assertEquals(2, selectedTab)
    }
}
