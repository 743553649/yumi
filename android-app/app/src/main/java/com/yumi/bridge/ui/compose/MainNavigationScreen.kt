package com.yumi.bridge.ui.compose

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

@Composable
fun MainNavigationScreen(
    state: HomeUiState,
    onModeSelected: (String) -> Unit,
    onAppModeChanged: (String, String) -> Unit,
    onClearLogs: () -> Unit,
    onRefreshLogs: () -> Unit = {},
    onTabSelected: (Int) -> Unit
) {
    Scaffold(
        bottomBar = {
            BottomNavigationBar(
                activeTab = state.activeTab,
                onTabSelected = { index ->
                    state.activeTab = index
                    onTabSelected(index)
                }
            )
        },
        containerColor = Color.Transparent
    ) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .fillMaxSize()
        ) {
            Box(modifier = Modifier.weight(1f)) {
                when (state.activeTab) {
                    0 -> HomeScreen(state = state, onModeSelected = onModeSelected)
                    1 -> LogScreen(state = state, onClearClick = onClearLogs, onRefreshClick = onRefreshLogs)
                    2 -> AppRulesScreen(state = state, onAppModeChanged = onAppModeChanged)
                }
            }
        }
    }
}

@Composable
private fun BottomNavigationBar(
    activeTab: Int,
    onTabSelected: (Int) -> Unit
) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 24.dp)
            .padding(bottom = 16.dp)
    ) {
        GlassBackdropWrapper(
            modifier = Modifier
                .fillMaxWidth()
                .height(64.dp),
            shape = RoundedCornerShape(32.dp)
        ) {
            NavigationBar(
                containerColor = Color.Transparent,
                tonalElevation = 0.dp
            ) {
                val tabs = listOf("首页" to 0, "日志" to 1, "应用" to 2)
                tabs.forEach { (title, index) ->
                    NavigationBarItem(
                        selected = activeTab == index,
                        onClick = { onTabSelected(index) },
                        icon = { Text(title) }
                    )
                }
            }
        }
    }
}

