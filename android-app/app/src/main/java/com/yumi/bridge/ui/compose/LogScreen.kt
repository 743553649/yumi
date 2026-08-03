package com.yumi.bridge.ui.compose

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.yumi.bridge.MainActivity
import com.yumi.bridge.ui.theme.YumiTheme

/**
 * Filter logs by specified filter level.
 */
fun filterLogs(
    logs: List<MainActivity.RealLogEntry>,
    filterLevel: Int
): List<MainActivity.RealLogEntry> {
    return logs.filter {
        filterLevel == MainActivity.LEVEL_ALL || it.level == filterLevel
    }
}

@Composable
fun LogScreen(
    state: HomeUiState,
    onClearClick: () -> Unit
) {
    val filteredLogs = remember(state.realLogs.toList(), state.currentFilterLevel) {
        filterLogs(state.realLogs, state.currentFilterLevel)
    }

    YumiTheme {
        GlassBackdropWrapper(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(16.dp)
            ) {
                LogHeader(onClearClick = onClearClick)

                Spacer(modifier = Modifier.height(8.dp))

                LogFilterChips(
                    currentFilterLevel = state.currentFilterLevel,
                    onFilterChange = { updateFilterLevel(it) }
                )

                Spacer(modifier = Modifier.height(16.dp))

                LogList(
                    logs = filteredLogs,
                    modifier = Modifier.weight(1f)
                )
            }
        }
    }
}

@Composable
private fun LogHeader(onClearClick: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = "运行日志",
            fontSize = 15.sp,
            fontWeight = FontWeight.Bold,
            color = Color(0xFF0F172A)
        )
        TextButton(onClick = onClearClick) {
            Text(text = "清除", color = Color(0xFF0284C7))
        }
    }
}

@Composable
private fun LogList(
    logs: List<MainActivity.RealLogEntry>,
    modifier: Modifier = Modifier
) {
    val listState = rememberLazyListState()

    LaunchedEffect(logs.size) {
        if (logs.isNotEmpty()) {
            listState.animateScrollToItem(logs.size - 1)
        }
    }

    LazyColumn(
        state = listState,
        modifier = modifier
            .fillMaxWidth()
            .background(Color(0x30FFFFFF), RoundedCornerShape(8.dp))
            .border(1.dp, Color(0x30FFFFFF), RoundedCornerShape(8.dp))
            .padding(8.dp)
    ) {
        items(logs) { log ->
            LogEntryItem(log)
        }
    }
}

@Composable
fun LogFilterChips(
    currentFilterLevel: Int,
    onFilterChange: (Int) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        val filters = listOf(
            MainActivity.LEVEL_ALL to "全部",
            MainActivity.LEVEL_DEBUG to "调试",
            MainActivity.LEVEL_INFO to "信息",
            MainActivity.LEVEL_WARN to "警告",
            MainActivity.LEVEL_ERROR to "错误"
        )

        filters.forEach { (level, title) ->
            val isSelected = currentFilterLevel == level
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(16.dp))
                    .clickable { onFilterChange(level) }
                    .background(
                        if (isSelected) Color(0xFF3B82F6) else Color(0x1A000000)
                    )
                    .border(
                        1.dp,
                        if (isSelected) Color.Transparent else Color(0x33000000),
                        RoundedCornerShape(16.dp)
                    )
                    .padding(horizontal = 12.dp, vertical = 6.dp)
            ) {
                Text(
                    text = title,
                    color = if (isSelected) Color.White else Color(0xFF475569),
                    fontSize = 12.sp,
                    fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal
                )
            }
        }
    }
}

@Composable
fun LogEntryItem(log: MainActivity.RealLogEntry) {
    val textColor = when (log.level) {
        MainActivity.LEVEL_ERROR -> Color(0xFFDC2626)
        MainActivity.LEVEL_WARN -> Color(0xFFD97706)
        MainActivity.LEVEL_DEBUG -> Color(0xFF94A3B8)
        MainActivity.LEVEL_INFO -> Color(0xFF334155)
        else -> Color(0xFF0F172A)
    }

    Text(
        text = log.formattedChineseLine,
        color = textColor,
        fontSize = 12.sp,
        modifier = Modifier.padding(vertical = 2.dp),
        lineHeight = 16.sp
    )
}
