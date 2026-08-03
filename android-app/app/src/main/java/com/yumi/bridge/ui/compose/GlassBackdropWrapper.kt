package com.yumi.bridge.ui.compose

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.dp

/**
 * 液态高斯模糊玻璃容器 (GlassBackdropWrapper)
 *
 * 采用 85% 冰白半透明渐变 (Ice-White Translucency Gradient)
 * 与极光柔蓝/纯白高光描边 (White/Soft-Blue Highlight Border)。
 */
@Composable
fun GlassBackdropWrapper(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(24.dp),
    content: @Composable BoxScope.() -> Unit
) {
    val highlightBorder = remember {
        BorderStroke(
            width = 1.5.dp,
            brush = Brush.linearGradient(
                colors = listOf(
                    Color(0xFFFFFFFF),
                    Color(0x80FFFFFF),
                    Color(0x400284C7)
                )
            )
        )
    }

    val glassBrush = remember {
        Brush.verticalGradient(
            colors = listOf(
                Color(0xD9FFFFFF),
                Color(0xB3E0F2FE)
            )
        )
    }

    Box(
        modifier = modifier
            .clip(shape)
            .background(brush = glassBrush)
            .border(highlightBorder, shape),
        content = content
    )
}
