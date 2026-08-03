package com.yumi.bridge.ui.compose

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
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
    modifier: Modifier = Modifier
) {
    GlassBackdropWrapper(modifier = modifier) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            // CPU 8-Core Grid (4 columns x 2 rows)
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                for (row in 0 until 2) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        for (col in 0 until 4) {
                            val coreIndex = row * 4 + col
                            CpuCoreCircularItem(
                                coreIndex = coreIndex,
                                freq = cpuFreqs.getOrElse(coreIndex) { 0L },
                                usage = cpuUsages.getOrElse(coreIndex) { 0 },
                                modifier = Modifier.weight(1f)
                            )
                        }
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
                        color = Color(0xFF0F172A),
                        fontWeight = FontWeight.SemiBold
                    )
                    Text(
                        text = "$ramDetailText (${ramPercent.coerceIn(0, 100)}%)",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF334155),
                        fontSize = 11.sp
                    )
                }

                val animatedRamProgress by animateFloatAsState(
                    targetValue = ramPercent.coerceIn(0, 100) / 100f,
                    animationSpec = tween(durationMillis = 300),
                    label = "RamProgress"
                )

                val ramColor = when {
                    ramPercent > 85 -> Color(0xFFDC2626)
                    ramPercent > 70 -> Color(0xFFEA580C)
                    else -> Color(0xFF0284C7)
                }

                LinearProgressIndicator(
                    progress = { animatedRamProgress },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(8.dp)
                        .clip(RoundedCornerShape(4.dp)),
                    color = ramColor,
                    trackColor = Color(0x30CBD5E1),
                )
            }
        }
    }
}

@Composable
private fun CpuCoreCircularItem(
    coreIndex: Int,
    freq: Long,
    usage: Int,
    modifier: Modifier = Modifier
) {
    val clampedUsage = usage.coerceIn(0, 100)

    val usageColor = when {
        clampedUsage > 80 -> Color(0xFFDC2626)
        clampedUsage > 60 -> Color(0xFFEA580C)
        clampedUsage > 30 -> Color(0xFF0284C7)
        else -> Color(0xFF16A34A)
    }

    val animatedUsageProgress by animateFloatAsState(
        targetValue = clampedUsage / 100f,
        animationSpec = tween(durationMillis = 300),
        label = "CpuCircularProgress"
    )

    val freqText = if (freq > 0L) "${freq}MHz" else "0MHz"

    Box(
        modifier = modifier
            .clip(RoundedCornerShape(12.dp))
            .background(Color(0x80F1F5F9))
            .border(1.dp, Color(0x300284C7), RoundedCornerShape(12.dp))
            .padding(vertical = 8.dp, horizontal = 2.dp),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            Text(
                text = "Core $coreIndex",
                style = MaterialTheme.typography.bodySmall,
                color = Color(0xFF0F172A),
                fontWeight = FontWeight.Bold,
                fontSize = 10.sp
            )

            // 环形进度图 (Circular Progress)
            Box(
                contentAlignment = Alignment.Center,
                modifier = Modifier.size(34.dp)
            ) {
                CircularProgressIndicator(
                    progress = { animatedUsageProgress },
                    modifier = Modifier.fillMaxSize(),
                    color = usageColor,
                    strokeWidth = 3.dp,
                    trackColor = Color(0x30CBD5E1)
                )
                Text(
                    text = "$clampedUsage%",
                    style = MaterialTheme.typography.labelSmall,
                    color = usageColor,
                    fontWeight = FontWeight.Bold,
                    fontSize = 9.sp
                )
            }

            Text(
                text = freqText,
                style = MaterialTheme.typography.labelSmall,
                color = Color(0xFF64748B),
                fontSize = 9.sp,
                fontWeight = FontWeight.Medium
            )
        }
    }
}
