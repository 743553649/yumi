package com.yumi.bridge.ui.compose

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun LiquidCpuDashboard(
    cpuFreqs: LongArray,
    cpuUsages: IntArray,
    ramPercent: Int,
    ramDetailText: String,
    uptimeText: String,
    modifier: Modifier = Modifier
) {
    GlassBackdropWrapper(modifier = modifier) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Header: Title & System Uptime
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Column {
                    Text(
                        text = "CPU & 内存监控",
                        style = MaterialTheme.typography.titleMedium,
                        color = Color.White,
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = "8-Core Realtime Monitor",
                        style = MaterialTheme.typography.labelSmall,
                        color = Color.White.copy(alpha = 0.5f)
                    )
                }

                // Uptime pill
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(12.dp))
                        .background(Color.White.copy(alpha = 0.1f))
                        .border(1.dp, Color.White.copy(alpha = 0.2f), RoundedCornerShape(12.dp))
                        .padding(horizontal = 10.dp, vertical = 4.dp)
                ) {
                    Text(
                        text = "运行: $uptimeText",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF00E5FF),
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Medium
                    )
                }
            }

            // CPU 8-Core Grid (2 columns x 4 rows)
            Text(
                text = "CPU 核心状态",
                style = MaterialTheme.typography.labelMedium,
                color = Color.White.copy(alpha = 0.7f),
                fontWeight = FontWeight.SemiBold
            )

            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                for (row in 0 until 4) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        val core1 = row * 2
                        val core2 = row * 2 + 1

                        CpuCoreItem(
                            coreIndex = core1,
                            freq = cpuFreqs.getOrElse(core1) { 0L },
                            usage = cpuUsages.getOrElse(core1) { 0 },
                            modifier = Modifier.weight(1f)
                        )
                        CpuCoreItem(
                            coreIndex = core2,
                            freq = cpuFreqs.getOrElse(core2) { 0L },
                            usage = cpuUsages.getOrElse(core2) { 0 },
                            modifier = Modifier.weight(1f)
                        )
                    }
                }
            }

            // RAM Progress Section
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        text = "RAM 内存使用",
                        style = MaterialTheme.typography.labelMedium,
                        color = Color.White.copy(alpha = 0.7f),
                        fontWeight = FontWeight.SemiBold
                    )
                    Text(
                        text = "$ramDetailText (${ramPercent.coerceIn(0, 100)}%)",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color.White.copy(alpha = 0.9f),
                        fontSize = 11.sp
                    )
                }

                val animatedRamProgress by animateFloatAsState(
                    targetValue = ramPercent.coerceIn(0, 100) / 100f,
                    animationSpec = tween(durationMillis = 300),
                    label = "RamProgress"
                )

                val ramColor = when {
                    ramPercent > 85 -> Color(0xFFEF4444)
                    ramPercent > 70 -> Color(0xFFF59E0B)
                    else -> Color(0xFF00E5FF)
                }

                LinearProgressIndicator(
                    progress = { animatedRamProgress },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(8.dp)
                        .clip(RoundedCornerShape(4.dp)),
                    color = ramColor,
                    trackColor = Color.White.copy(alpha = 0.1f),
                )
            }
        }
    }
}

@Composable
private fun CpuCoreItem(
    coreIndex: Int,
    freq: Long,
    usage: Int,
    modifier: Modifier = Modifier
) {
    val clampedUsage = usage.coerceIn(0, 100)

    val usageColor = when {
        clampedUsage > 80 -> Color(0xFFEF4444)
        clampedUsage > 60 -> Color(0xFFF59E0B)
        clampedUsage > 30 -> Color(0xFF06B6D4)
        else -> Color(0xFF10B981)
    }

    val animatedUsageProgress by animateFloatAsState(
        targetValue = clampedUsage / 100f,
        animationSpec = tween(durationMillis = 300),
        label = "CpuUsageProgress"
    )

    val freqText = when {
        freq > 100000L -> "${freq / 1000L} MHz"
        freq > 0L -> "$freq MHz"
        else -> "0 MHz"
    }

    Box(
        modifier = modifier
            .clip(RoundedCornerShape(12.dp))
            .background(Color.White.copy(alpha = 0.05f))
            .border(1.dp, Color.White.copy(alpha = 0.1f), RoundedCornerShape(12.dp))
            .padding(8.dp)
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(
                    text = "Core $coreIndex",
                    style = MaterialTheme.typography.bodySmall,
                    color = Color.White.copy(alpha = 0.9f),
                    fontWeight = FontWeight.Bold,
                    fontSize = 11.sp
                )
                Text(
                    text = "$clampedUsage%",
                    style = MaterialTheme.typography.bodySmall,
                    color = usageColor,
                    fontWeight = FontWeight.Bold,
                    fontSize = 11.sp
                )
            }

            LinearProgressIndicator(
                progress = { animatedUsageProgress },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(4.dp)
                    .clip(RoundedCornerShape(2.dp)),
                color = usageColor,
                trackColor = Color.White.copy(alpha = 0.1f),
            )

            Text(
                text = freqText,
                style = MaterialTheme.typography.labelSmall,
                color = Color.White.copy(alpha = 0.5f),
                fontSize = 9.sp
            )
        }
    }
}
