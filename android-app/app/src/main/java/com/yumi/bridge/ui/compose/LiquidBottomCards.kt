package com.yumi.bridge.ui.compose

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
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
fun LiquidBottomCards(
    ramDetailText: String,
    ramPercent: Int,
    swapDetailText: String,
    swapPercent: Int,
    batteryLevel: Int,
    batteryTempText: String,
    batteryPowerText: String,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        // 卡片 1: RAM 内存卡片
        RamInfoCard(
            ramDetailText = ramDetailText,
            ramPercent = ramPercent,
            swapDetailText = swapDetailText,
            swapPercent = swapPercent,
            modifier = Modifier.weight(1f)
        )

        // 卡片 2: 电池信息卡片
        BatteryInfoCard(
            batteryLevel = batteryLevel,
            batteryTempText = batteryTempText,
            batteryPowerText = batteryPowerText,
            modifier = Modifier.weight(1f)
        )
    }
}

@Composable
private fun RamInfoCard(
    ramDetailText: String,
    ramPercent: Int,
    swapDetailText: String,
    swapPercent: Int,
    modifier: Modifier = Modifier
) {
    GlassBackdropWrapper(modifier = modifier) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            Text(
                text = "RAM 内存",
                style = MaterialTheme.typography.titleSmall,
                color = Color(0xFF0F172A),
                fontWeight = FontWeight.Bold,
                fontSize = 13.sp
            )

            // 物理内存
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "物理内存",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF475569),
                        fontSize = 10.sp,
                        fontWeight = FontWeight.Medium
                    )
                    Text(
                        text = "$ramDetailText (${ramPercent.coerceIn(0, 100)}%)",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF0284C7),
                        fontSize = 10.sp,
                        fontWeight = FontWeight.Bold
                    )
                }

                val animatedRamProgress by animateFloatAsState(
                    targetValue = ramPercent.coerceIn(0, 100) / 100f,
                    animationSpec = tween(durationMillis = 300),
                    label = "RamProgress"
                )

                LinearProgressIndicator(
                    progress = { animatedRamProgress },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(5.dp)
                        .clip(RoundedCornerShape(3.dp)),
                    color = Color(0xFF0284C7),
                    trackColor = Color(0x30CBD5E1)
                )
            }

            // 交换分区内存 (Swap / ZRAM)
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "交换分区",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF475569),
                        fontSize = 10.sp,
                        fontWeight = FontWeight.Medium
                    )
                    Text(
                        text = "$swapDetailText (${swapPercent.coerceIn(0, 100)}%)",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF9333EA),
                        fontSize = 10.sp,
                        fontWeight = FontWeight.Bold
                    )
                }

                val animatedSwapProgress by animateFloatAsState(
                    targetValue = swapPercent.coerceIn(0, 100) / 100f,
                    animationSpec = tween(durationMillis = 300),
                    label = "SwapProgress"
                )

                LinearProgressIndicator(
                    progress = { animatedSwapProgress },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(5.dp)
                        .clip(RoundedCornerShape(3.dp)),
                    color = Color(0xFF9333EA),
                    trackColor = Color(0x30CBD5E1)
                )
            }
        }
    }
}

@Composable
private fun BatteryInfoCard(
    batteryLevel: Int,
    batteryTempText: String,
    batteryPowerText: String,
    modifier: Modifier = Modifier
) {
    val clampedLevel = batteryLevel.coerceIn(0, 100)
    val batteryColor = when {
        clampedLevel <= 20 -> Color(0xFFDC2626)
        clampedLevel <= 40 -> Color(0xFFEA580C)
        else -> Color(0xFF16A34A)
    }

    GlassBackdropWrapper(modifier = modifier) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "电池信息",
                    style = MaterialTheme.typography.titleSmall,
                    color = Color(0xFF0F172A),
                    fontWeight = FontWeight.Bold,
                    fontSize = 13.sp
                )
                Text(
                    text = "$clampedLevel%",
                    style = MaterialTheme.typography.titleSmall,
                    color = batteryColor,
                    fontWeight = FontWeight.Bold,
                    fontSize = 12.sp
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(
                        text = "实时功率",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF475569),
                        fontSize = 10.sp
                    )
                    Text(
                        text = batteryPowerText,
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF0F172A),
                        fontWeight = FontWeight.Bold,
                        fontSize = 11.sp
                    )
                }

                Column(
                    horizontalAlignment = Alignment.End,
                    verticalArrangement = Arrangement.spacedBy(2.dp)
                ) {
                    Text(
                        text = "电池温度",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF475569),
                        fontSize = 10.sp
                    )
                    Text(
                        text = batteryTempText,
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF0F172A),
                        fontWeight = FontWeight.Bold,
                        fontSize = 11.sp
                    )
                }
            }
        }
    }
}
