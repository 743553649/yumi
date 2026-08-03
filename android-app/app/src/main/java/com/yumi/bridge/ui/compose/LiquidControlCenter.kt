package com.yumi.bridge.ui.compose

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
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

data class ModeConfig(
    val key: String,
    val title: String,
    val subtitle: String,
    val accentColor: Color
)

private val MODES = listOf(
    ModeConfig("powersave", "省电", "低功耗运行", Color(0xFF16A34A)),
    ModeConfig("balance", "平衡", "标准预设", Color(0xFF0284C7)),
    ModeConfig("performance", "性能", "高吞吐量", Color(0xFFEA580C)),
    ModeConfig("fast", "极速", "超低延迟", Color(0xFFDC2626)),
    ModeConfig("fas", "FAS 帧感知", "游戏逐帧调频", Color(0xFF9333EA))
)

@Composable
fun LiquidControlCenter(
    currentMode: String,
    onModeSelected: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    GlassBackdropWrapper(modifier = modifier) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(
                    text = "性能模式中心",
                    style = MaterialTheme.typography.titleMedium,
                    color = Color(0xFF0F172A),
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Liquid Control",
                    style = MaterialTheme.typography.labelSmall,
                    color = Color(0xFF64748B)
                )
            }

            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    ModeCard(
                        config = MODES[0],
                        isSelected = currentMode.equals(MODES[0].key, ignoreCase = true),
                        onClick = { onModeSelected(MODES[0].key) },
                        modifier = Modifier.weight(1f)
                    )
                    ModeCard(
                        config = MODES[1],
                        isSelected = currentMode.equals(MODES[1].key, ignoreCase = true),
                        onClick = { onModeSelected(MODES[1].key) },
                        modifier = Modifier.weight(1f)
                    )
                }
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    ModeCard(
                        config = MODES[2],
                        isSelected = currentMode.equals(MODES[2].key, ignoreCase = true),
                        onClick = { onModeSelected(MODES[2].key) },
                        modifier = Modifier.weight(1f)
                    )
                    ModeCard(
                        config = MODES[3],
                        isSelected = currentMode.equals(MODES[3].key, ignoreCase = true),
                        onClick = { onModeSelected(MODES[3].key) },
                        modifier = Modifier.weight(1f)
                    )
                }
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    ModeCard(
                        config = MODES[4],
                        isSelected = currentMode.equals(MODES[4].key, ignoreCase = true),
                        onClick = { onModeSelected(MODES[4].key) },
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            }
        }
    }
}

@Composable
private fun ModeCard(
    config: ModeConfig,
    isSelected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val animatedBgColor by animateColorAsState(
        targetValue = if (isSelected) config.accentColor.copy(alpha = 0.15f) else Color(0x80F1F5F9),
        animationSpec = tween(durationMillis = 250),
        label = "ModeCardBg"
    )

    val cardShape = RoundedCornerShape(16.dp)

    Box(
        modifier = modifier
            .clip(cardShape)
            .background(animatedBgColor)
            .border(
                width = if (isSelected) 1.5.dp else 1.dp,
                color = if (isSelected) config.accentColor else Color(0x40CBD5E1),
                shape = cardShape
            )
            .clickable(onClick = onClick)
            .padding(vertical = 12.dp, horizontal = 14.dp)
    ) {
        Column {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(
                    text = config.title,
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    color = if (isSelected) config.accentColor else Color(0xFF0F172A)
                )
                if (isSelected) {
                    Box(
                        modifier = Modifier
                            .size(8.dp)
                            .clip(CircleShape)
                            .background(config.accentColor)
                    )
                }
            }
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = config.subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = Color(0xFF475569),
                fontSize = 11.sp
            )
        }
    }
}
