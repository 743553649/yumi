package com.yumi.bridge.ui.compose

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.yumi.bridge.ui.theme.YumiTheme
import com.kyant.backdrop.backdrops.LayerBackdrop
import com.kyant.backdrop.backdrops.layerBackdrop
import com.kyant.backdrop.backdrops.rememberLayerBackdrop

// Global shared backdrop state
val sharedBackdropState = mutableStateOf<LayerBackdrop?>(null)


class HomeUiState {
    var currentMode by mutableStateOf("balance")
    var cpuFreqs by mutableStateOf(LongArray(8))
    var cpuUsages by mutableStateOf(IntArray(8))

    // RAM & Swap Metrics
    var ramPercent by mutableStateOf(0)
    var ramDetailText by mutableStateOf("0.0G / 0.0G")
    var swapPercent by mutableStateOf(0)
    var swapDetailText by mutableStateOf("0.0G / 0.0G")

    // Battery Metrics
    var batteryLevel by mutableStateOf(100)
    var batteryTempText by mutableStateOf("0.0 ℃")
    var batteryPowerText by mutableStateOf("0.0 W")

    var uptimeText by mutableStateOf("00:00:00")
    var isDaemonOnline by mutableStateOf(true)

    // Logs state
    val realLogs = mutableStateListOf<com.yumi.bridge.MainActivity.RealLogEntry>()
    var currentFilterLevel by mutableStateOf(com.yumi.bridge.MainActivity.LEVEL_ALL)

    // Apps state
    val installedApps = mutableStateListOf<com.yumi.bridge.MainActivity.AppRuleItem>()
    var appSearchQuery by mutableStateOf("")

    // Tab State
    var activeTab by mutableStateOf(0)
}

private val globalHomeState = HomeUiState()

@Composable
fun YumiTopHeaderCard(
    uptimeText: String,
    isOnline: Boolean = true,
    modifier: Modifier = Modifier
) {
    GlassBackdropWrapper(modifier = modifier) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            // Display title
            Text(
                text = "yumi 调度",
                style = MaterialTheme.typography.titleMedium,
                color = Color(0xFF0F172A),
                fontWeight = FontWeight.Bold,
                fontSize = 16.sp
            )

            // Display online status and uptime
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                // Online status tag
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(12.dp))
                        .background(if (isOnline) Color(0x2022C55E) else Color(0x20EF4444))
                        .border(
                            1.dp,
                            if (isOnline) Color(0x8022C55E) else Color(0x80EF4444),
                            RoundedCornerShape(12.dp)
                        )
                        .padding(horizontal = 8.dp, vertical = 4.dp)
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        Box(
                            modifier = Modifier
                                .size(6.dp)
                                .clip(CircleShape)
                                .background(if (isOnline) Color(0xFF16A34A) else Color(0xFFDC2626))
                        )
                        Text(
                            text = if (isOnline) "在线" else "离线",
                            style = MaterialTheme.typography.bodySmall,
                            color = if (isOnline) Color(0xFF15803D) else Color(0xFFB91C1C),
                            fontSize = 11.sp,
                            fontWeight = FontWeight.SemiBold
                        )
                    }
                }

                // Uptime tag
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(12.dp))
                        .background(Color(0x30E0F2FE))
                        .border(1.dp, Color(0x800284C7), RoundedCornerShape(12.dp))
                        .padding(horizontal = 8.dp, vertical = 4.dp)
                ) {
                    Text(
                        text = "运行: $uptimeText",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF0284C7),
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Medium
                    )
                }
            }
        }
    }
}

@Composable
fun HomeScreen(
    state: HomeUiState,
    onModeSelected: (String) -> Unit
) {
    YumiTheme {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .windowInsetsPadding(WindowInsets.statusBars)
                .padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // 1. Top header card
            YumiTopHeaderCard(
                uptimeText = state.uptimeText,
                isOnline = state.isDaemonOnline
            )

            // 2. Performance mode card
            LiquidControlCenter(
                currentMode = state.currentMode,
                onModeSelected = onModeSelected
            )

            // 3. CPU dashboard
            LiquidCpuDashboard(
                cpuFreqs = state.cpuFreqs,
                cpuUsages = state.cpuUsages
            )

            // 4. Bottom cards (RAM & Swap, Battery)
            LiquidBottomCards(
                ramDetailText = state.ramDetailText,
                ramPercent = state.ramPercent,
                swapDetailText = state.swapDetailText,
                swapPercent = state.swapPercent,
                batteryLevel = state.batteryLevel,
                batteryTempText = state.batteryTempText,
                batteryPowerText = state.batteryPowerText
            )
        }
    }
}

/**
 * Initialize global background ComposeView
 */
fun attachBackgroundHost(composeView: ComposeView) {
    composeView.setViewCompositionStrategy(
        androidx.compose.ui.platform.ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed
    )
    composeView.setContent {
        YumiTheme {
            val backdrop = rememberLayerBackdrop()
            DisposableEffect(backdrop) {
                sharedBackdropState.value = backdrop
                onDispose {
                    if (sharedBackdropState.value == backdrop) {
                        sharedBackdropState.value = null
                    }
                }
            }
            LiquidMeshBackground(
                modifier = Modifier.layerBackdrop(backdrop)
            ) { }
        }
    }
}

fun interface OnClearLogsListener {
    fun onClearLogs()
}

fun interface OnRefreshLogsListener {
    fun onRefreshLogs()
}

fun interface OnFilterLevelChangeListener {
    fun onFilterLevelChanged(level: Int)
}

private var filterLevelChangeListener: OnFilterLevelChangeListener? = null

fun attachMainScreen(
    composeView: ComposeView,
    onModeSelectedListener: OnModeSelectedListener,
    onAppModeChangedListener: AppRuleChangeListener,
    onTabSelectedListener: OnTabSelectedListener,
    onRefreshLogsListener: OnRefreshLogsListener = OnRefreshLogsListener { },
    onClearLogsListener: OnClearLogsListener = OnClearLogsListener { },
    onFilterLevelChangeListener: OnFilterLevelChangeListener = OnFilterLevelChangeListener { }
) {
    filterLevelChangeListener = onFilterLevelChangeListener
    composeView.setViewCompositionStrategy(
        androidx.compose.ui.platform.ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed
    )
    composeView.setContent {
        YumiTheme {
            MainNavigationScreen(
                state = globalHomeState,
                onModeSelected = { onModeSelectedListener.onModeSelected(it) },
                onAppModeChanged = { pkg, mode -> onAppModeChangedListener.onAppModeChanged(pkg, mode) },
                onClearLogs = {
                    globalHomeState.realLogs.clear()
                    onClearLogsListener.onClearLogs()
                },
                onRefreshLogs = { onRefreshLogsListener.onRefreshLogs() },
                onTabSelected = { onTabSelectedListener.onTabSelected(it) }
            )
        }
    }
}

/**
 * Update function for Java background polling to drive Compose State.
 */
@JvmOverloads
fun updateHomeScreenState(
    currentMode: String,
    cpuFreqs: LongArray,
    cpuUsages: IntArray,
    ramPercent: Int,
    ramDetailText: String,
    swapPercent: Int,
    swapDetailText: String,
    batteryLevel: Int,
    batteryTempText: String,
    batteryPowerText: String,
    uptimeText: String,
    isDaemonOnline: Boolean = true
) {
    globalHomeState.currentMode = currentMode
    globalHomeState.cpuFreqs = cpuFreqs.copyOf()
    globalHomeState.cpuUsages = cpuUsages.copyOf()
    globalHomeState.ramPercent = ramPercent
    globalHomeState.ramDetailText = ramDetailText
    globalHomeState.swapPercent = swapPercent
    globalHomeState.swapDetailText = swapDetailText
    globalHomeState.batteryLevel = batteryLevel
    globalHomeState.batteryTempText = batteryTempText
    globalHomeState.batteryPowerText = batteryPowerText
    globalHomeState.uptimeText = uptimeText
    globalHomeState.isDaemonOnline = isDaemonOnline
}

fun updateLogState(
    logs: List<com.yumi.bridge.MainActivity.RealLogEntry>,
    filterLevel: Int
) {
    globalHomeState.realLogs.clear()
    globalHomeState.realLogs.addAll(logs)
    globalHomeState.currentFilterLevel = filterLevel
}

fun clearLogState() {
    globalHomeState.realLogs.clear()
}

fun updateFilterLevel(level: Int) {
    globalHomeState.currentFilterLevel = level
    filterLevelChangeListener?.onFilterLevelChanged(level)
}

fun updateInstalledApps(apps: List<com.yumi.bridge.MainActivity.AppRuleItem>) {
    globalHomeState.installedApps.clear()
    globalHomeState.installedApps.addAll(apps)
}

fun interface OnModeSelectedListener {
    fun onModeSelected(mode: String)
}

interface AppRuleChangeListener {
    fun onAppModeChanged(packageName: String, newMode: String)
}

fun interface OnTabSelectedListener {
    fun onTabSelected(index: Int)
}
