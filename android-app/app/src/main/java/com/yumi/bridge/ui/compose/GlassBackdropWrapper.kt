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
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/**
 * 液态高斯模糊玻璃容器 (GlassBackdropWrapper)
 *
 * 采用 30% - 42% 冰白半透明水纹渐变 (Translucent Ice-White Water Ripple)
 * 与 24dp 硬件高斯模糊 (24dp Backdrop Blur)、45° 斜角高光描边 (45-degree Refractive Border)
 * 以及 8dp 弥散软阴影 (Soft Ambient Drop Shadow)。
 */
@Composable
fun GlassBackdropWrapper(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(24.dp),
    blurRadius: Dp = 24.dp,
    content: @Composable BoxScope.() -> Unit
) {
    val highlightBorder = remember {
        BorderStroke(
            width = 1.2.dp,
            brush = Brush.linearGradient(
                colors = listOf(
                    Color(0xE6FFFFFF), // 顶部 90% 不透明强日光高光
                    Color(0x33FFFFFF), // 中段 20% 通透过渡
                    Color(0x500284C7)  // 底部天蓝折射
                ),
                start = Offset(0f, 0f),
                end = Offset(Float.POSITIVE_INFINITY, Float.POSITIVE_INFINITY)
            )
        )
    }

    val glassBrush = remember {
        Brush.verticalGradient(
            colors = listOf(
                Color(0x59FFFFFF), // 35% 白
                Color(0x3DDCFCE7)  // 24% 浅青透光
            )
        )
    }

    Box(
        modifier = modifier
            .shadow(elevation = 8.dp, shape = shape, spotColor = Color(0x1A0284C7))
            .clip(shape)
            .blur(radius = blurRadius)
            .background(brush = glassBrush)
            .border(highlightBorder, shape),
        content = content
    )
}
