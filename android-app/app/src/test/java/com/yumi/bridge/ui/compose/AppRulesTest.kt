package com.yumi.bridge.ui.compose

import com.yumi.bridge.MainActivity
import org.junit.Assert.assertEquals
import org.junit.Test

class AppRulesTest {

    @Test
    fun filterAppRules_emptyQuery_returnsAllApps() {
        val apps = listOf(
            MainActivity.AppRuleItem("com.example.game", "Game App", "balance"),
            MainActivity.AppRuleItem("com.example.chat", "Chat App", "powersave")
        )
        val filtered = filterAppRules(apps, "")
        assertEquals(2, filtered.size)
    }

    @Test
    fun filterAppRules_matchingQuery_returnsFilteredApps() {
        val apps = listOf(
            MainActivity.AppRuleItem("com.example.game", "Game App", "balance"),
            MainActivity.AppRuleItem("com.example.chat", "Chat App", "powersave")
        )
        val filteredByApp = filterAppRules(apps, "game")
        assertEquals(1, filteredByApp.size)
        assertEquals("com.example.game", filteredByApp[0].packageName)

        val filteredByPkg = filterAppRules(apps, "chat")
        assertEquals(1, filteredByPkg.size)
        assertEquals("com.example.chat", filteredByPkg[0].packageName)
    }
}
