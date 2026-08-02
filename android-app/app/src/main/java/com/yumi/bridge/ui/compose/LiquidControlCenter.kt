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
import androidx.compose.ui.graphics.Brush
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
    ModeConfig("powersave", "省电", "续航优先", Color(0xFF10B981)),
    ModeConfig("balance", "平衡", "均衡流畅", Color(0xFF06B6D4)),
    ModeConfig("performance", "性能", "高刷响应", Color(0xFFF59E0B)),
    ModeConfig("ultra", "极速", "极限释放", Color(0xFFEF4444))
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
                    color = Color.White,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Liquid Control",
                    style = MaterialTheme.typography.labelSmall,
                    color = Color.White.copy(alpha = 0.5f)
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
        targetValue = if (isSelected) config.accentColor.copy(alpha = 0.22f) else Color.White.copy(alpha = 0.05f),
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
                brush = if (isSelected) {
                    Brush.linearGradient(
                        colors = listOf(
                            config.accentColor,
                            config.accentColor.copy(alpha = 0.4f)
                        )
                    )
                } else {
                    Brush.linearGradient(
                        colors = listOf(
                            Color.White.copy(alpha = 0.2f),
                            Color.White.copy(alpha = 0.05f)
                        )
                    )
                },
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
                    color = if (isSelected) config.accentColor else Color.White
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
                color = Color.White.copy(alpha = 0.65f),
                fontSize = 11.sp
            )
        }
    }
}
