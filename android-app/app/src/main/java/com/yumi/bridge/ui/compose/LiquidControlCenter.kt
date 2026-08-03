package com.yumi.bridge.ui.compose

import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

data class ModeConfig(
    val key: String,
    val title: String,
    val subtitle: String,
    val accentColor: Color,
    val icon: ImageVector
)

// 1. 省电 (Powersave) - 电池/节能图标
private val PowersaveIcon: ImageVector = ImageVector.Builder(
    name = "PowersaveIcon",
    defaultWidth = 16.dp,
    defaultHeight = 16.dp,
    viewportWidth = 24f,
    viewportHeight = 24f
).path(fill = SolidColor(Color(0xFF16A34A))) {
    moveTo(17f, 4f)
    horizontalLineTo(7f)
    lineTo(7f, 20f)
    horizontalLineTo(17f)
    lineTo(17f, 4f)
    close()
    moveTo(11f, 16f)
    verticalLineTo(13f)
    horizontalLineTo(9f)
    lineTo(13f, 8f)
    verticalLineTo(11f)
    horizontalLineTo(15f)
    lineTo(11f, 16f)
    close()
}.build()

// 2. 平衡 (Balance) - 天平/均衡器图标
private val BalanceIcon: ImageVector = ImageVector.Builder(
    name = "BalanceIcon",
    defaultWidth = 16.dp,
    defaultHeight = 16.dp,
    viewportWidth = 24f,
    viewportHeight = 24f
).path(fill = SolidColor(Color(0xFF0284C7))) {
    moveTo(10f, 20f)
    horizontalLineTo(14f)
    verticalLineTo(4f)
    horizontalLineTo(10f)
    verticalLineTo(20f)
    close()
    moveTo(4f, 20f)
    horizontalLineTo(8f)
    verticalLineTo(10f)
    horizontalLineTo(4f)
    verticalLineTo(20f)
    close()
    moveTo(16f, 14f)
    verticalLineTo(20f)
    horizontalLineTo(20f)
    verticalLineTo(14f)
    horizontalLineTo(16f)
    close()
}.build()

// 3. 性能 (Performance) - 仪表盘/高吞吐图标
private val PerformanceIcon: ImageVector = ImageVector.Builder(
    name = "PerformanceIcon",
    defaultWidth = 16.dp,
    defaultHeight = 16.dp,
    viewportWidth = 24f,
    viewportHeight = 24f
).path(fill = SolidColor(Color(0xFFEA580C))) {
    moveTo(12f, 3f)
    lineTo(4f, 19f)
    horizontalLineTo(20f)
    lineTo(12f, 3f)
    close()
    moveTo(11f, 9f)
    horizontalLineTo(13f)
    verticalLineTo(14f)
    horizontalLineTo(11f)
    verticalLineTo(9f)
    close()
    moveTo(11f, 16f)
    horizontalLineTo(13f)
    verticalLineTo(18f)
    horizontalLineTo(11f)
    verticalLineTo(16f)
    close()
}.build()

// 4. 极速 (Fast) - 闪电/极速图标
private val FastIcon: ImageVector = ImageVector.Builder(
    name = "FastIcon",
    defaultWidth = 16.dp,
    defaultHeight = 16.dp,
    viewportWidth = 24f,
    viewportHeight = 24f
).path(fill = SolidColor(Color(0xFFDC2626))) {
    moveTo(7f, 2f)
    verticalLineTo(13f)
    horizontalLineTo(10f)
    verticalLineTo(22f)
    lineTo(17f, 10f)
    horizontalLineTo(13f)
    lineTo(17f, 2f)
    horizontalLineTo(7f)
    close()
}.build()

val SUPPORTED_MODES = listOf(
    ModeConfig("powersave", "省电", "低功耗运行", Color(0xFF16A34A), PowersaveIcon),
    ModeConfig("balance", "平衡", "标准预设", Color(0xFF0284C7), BalanceIcon),
    ModeConfig("performance", "性能", "高吞吐量", Color(0xFFEA580C), PerformanceIcon),
    ModeConfig("fast", "极速", "超低延迟", Color(0xFFDC2626), FastIcon)
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
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                ModeCard(
                    config = SUPPORTED_MODES[0],
                    isSelected = currentMode.equals(SUPPORTED_MODES[0].key, ignoreCase = true),
                    onClick = { onModeSelected(SUPPORTED_MODES[0].key) },
                    modifier = Modifier.weight(1f)
                )
                ModeCard(
                    config = SUPPORTED_MODES[1],
                    isSelected = currentMode.equals(SUPPORTED_MODES[1].key, ignoreCase = true),
                    onClick = { onModeSelected(SUPPORTED_MODES[1].key) },
                    modifier = Modifier.weight(1f)
                )
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                ModeCard(
                    config = SUPPORTED_MODES[2],
                    isSelected = currentMode.equals(SUPPORTED_MODES[2].key, ignoreCase = true),
                    onClick = { onModeSelected(SUPPORTED_MODES[2].key) },
                    modifier = Modifier.weight(1f)
                )
                ModeCard(
                    config = SUPPORTED_MODES[3],
                    isSelected = currentMode.equals(SUPPORTED_MODES[3].key, ignoreCase = true),
                    onClick = { onModeSelected(SUPPORTED_MODES[3].key) },
                    modifier = Modifier.weight(1f)
                )
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
    val cardShape = RoundedCornerShape(16.dp)

    // Highlight background: linear-gradient(120deg, #a1c4fd 0%, #c2e9fb 100%)
    val backgroundModifier = if (isSelected) {
        Modifier.background(
            brush = Brush.linearGradient(
                colors = listOf(Color(0xFFA1C4FD), Color(0xFFC2E9FB))
            )
        )
    } else {
        Modifier.background(Color(0x80F1F5F9))
    }

    Box(
        modifier = modifier
            .clip(cardShape)
            .then(backgroundModifier)
            .border(
                width = if (isSelected) 1.5.dp else 1.dp,
                color = if (isSelected) Color(0xFF749BEB) else Color(0x40CBD5E1),
                shape = cardShape
            )
            .clickable(onClick = onClick)
            .padding(vertical = 12.dp, horizontal = 12.dp)
    ) {
        Column {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    Icon(
                        imageVector = config.icon,
                        contentDescription = null,
                        tint = if (isSelected) config.accentColor else Color(0xFF64748B),
                        modifier = Modifier.size(16.dp)
                    )
                    Text(
                        text = config.title,
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                        color = Color(0xFF0F172A)
                    )
                }
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
