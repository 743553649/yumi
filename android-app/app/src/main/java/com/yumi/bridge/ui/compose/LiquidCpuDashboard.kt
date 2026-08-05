package com.yumi.bridge.ui.compose

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
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
        clampedUsage > 85 -> Color(0xFFF43F5E) // Soft Coral Red (High Load)
        clampedUsage > 70 -> Color(0xFFF59E0B) // Warm Amber Orange (Medium-High Load)
        clampedUsage > 40 -> Color(0xFF0284C7) // Ocean Electric Blue (Medium Load)
        else -> Color(0xFF38BDF8)               // Ice Sky Cyan (Low Load - Calm)
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
            .background(Color(0x14FFFFFF))
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

            // 环形进度图 (Harmonized Ice Circular Progress)
            Box(
                contentAlignment = Alignment.Center,
                modifier = Modifier.size(34.dp)
            ) {
                CircularProgressIndicator(
                    progress = { animatedUsageProgress },
                    modifier = Modifier.fillMaxSize(),
                    color = usageColor,
                    strokeWidth = 2.5.dp,
                    trackColor = Color(0x1F0284C7)
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
