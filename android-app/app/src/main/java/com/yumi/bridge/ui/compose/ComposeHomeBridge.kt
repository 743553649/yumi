package com.yumi.bridge.ui.compose

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.unit.dp
import com.yumi.bridge.ui.theme.YumiTheme

class HomeUiState {
    var currentMode by mutableStateOf("balance")
    var cpuFreqs by mutableStateOf(LongArray(8))
    var cpuUsages by mutableStateOf(IntArray(8))
    var ramPercent by mutableStateOf(0)
    var ramDetailText by mutableStateOf("已用 0.0G / 0.0G")
    var uptimeText by mutableStateOf("00:00:00")
}

private val globalHomeState = HomeUiState()

@Composable
fun HomeScreen(
    state: HomeUiState,
    onModeSelected: (String) -> Unit
) {
    YumiTheme {
        LiquidMeshBackground {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(16.dp)
            ) {
                LiquidControlCenter(
                    currentMode = state.currentMode,
                    onModeSelected = onModeSelected
                )
                Spacer(modifier = Modifier.height(16.dp))
                LiquidCpuDashboard(
                    cpuFreqs = state.cpuFreqs,
                    cpuUsages = state.cpuUsages,
                    ramPercent = state.ramPercent,
                    ramDetailText = state.ramDetailText,
                    uptimeText = state.uptimeText
                )
            }
        }
    }
}

/**
 * 初始化 ComposeView 渲染树（仅需在 Java 中调用一次）。
 */
fun attachHomeScreen(
    composeView: ComposeView,
    onModeSelectedListener: OnModeSelectedListener
) {
    composeView.setContent {
        HomeScreen(
            state = globalHomeState,
            onModeSelected = { mode -> onModeSelectedListener.onModeSelected(mode) }
        )
    }
}

/**
 * 供 Java 后台轮询调用的更新函数，驱动 Compose State 响应式重绘，避免创建新对象与频繁 GC。
 */
fun updateHomeScreenState(
    currentMode: String,
    cpuFreqs: LongArray,
    cpuUsages: IntArray,
    ramPercent: Int,
    ramDetailText: String,
    uptimeText: String
) {
    globalHomeState.currentMode = currentMode
    globalHomeState.cpuFreqs = cpuFreqs.copyOf()
    globalHomeState.cpuUsages = cpuUsages.copyOf()
    globalHomeState.ramPercent = ramPercent
    globalHomeState.ramDetailText = ramDetailText
    globalHomeState.uptimeText = uptimeText
}

fun interface OnModeSelectedListener {
    fun onModeSelected(mode: String)
}
