package com.yumi.bridge.ui.compose

import android.os.Build
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
import io.github.kyant0.backdrop.Backdrop

/**
 * 液态高斯模糊玻璃容器 (GlassBackdropWrapper)
 *
 * 在 Android 13+ (API 33+) 采用 io.github.kyant0.backdrop.Backdrop 极光模糊与 Shader 滤镜，
 * 在 API 26-32 环境优雅降级为半透明冰白混色圆角卡片与精细高光双色渐变描边。
 */
@Composable
fun GlassBackdropWrapper(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(24.dp),
    content: @Composable BoxScope.() -> Unit
) {
    val highlightBorder = remember {
        BorderStroke(
            width = 1.dp,
            brush = Brush.linearGradient(
                colors = listOf(
                    Color(0x66FFFFFF), // Ice white top-left highlight
                    Color(0x1AFFFFFF), // Mid translucent body
                    Color(0x4000E5FF)  // Neon cyan bottom-right reflection
                )
            )
        )
    }

    val fallbackBrush = remember {
        Brush.verticalGradient(
            colors = listOf(
                Color(0xD9F8FAFC), // 85% ice-white frosted top blend
                Color(0xB3E2E8F0)  // 70% soft slate frosted bottom blend
            )
        )
    }

    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        // API 33+: Use Kyant Backdrop for real-time liquid blur and hardware acceleration
        Backdrop(
            modifier = modifier
                .clip(shape)
                .border(highlightBorder, shape),
            shape = shape,
            color = Color(0x26FFFFFF)
        ) {
            content()
        }
    } else {
        // API 26-32 Fallback: Translucent ice-glass background with fine border highlight
        Box(
            modifier = modifier
                .clip(shape)
                .background(brush = fallbackBrush)
                .border(highlightBorder, shape),
            content = content
        )
    }
}
