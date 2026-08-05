package com.yumi.bridge.ui.compose

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import com.yumi.bridge.R
import kotlin.math.cos
import kotlin.math.sin

private val Blob1Colors = listOf(
    Color(0xFF0284C7).copy(alpha = 0.25f),
    Color(0xFF0284C7).copy(alpha = 0.10f),
    Color.Transparent
)

private val Blob2Colors = listOf(
    Color(0xFF38BDF8).copy(alpha = 0.28f),
    Color(0xFF38BDF8).copy(alpha = 0.12f),
    Color.Transparent
)

private val Blob3Colors = listOf(
    Color(0xFF60A5FA).copy(alpha = 0.22f),
    Color(0xFF60A5FA).copy(alpha = 0.08f),
    Color.Transparent
)

private val Blob4Colors = listOf(
    Color(0xFF2DD4BF).copy(alpha = 0.20f),
    Color(0xFF2DD4BF).copy(alpha = 0.08f),
    Color.Transparent
)

private val ScrimColors = listOf(
    Color(0x05FFFFFF),
    Color(0x10E0F2FE),
    Color(0x20BAE6FD)
)

/**
 * 动态彩色流体天幕背景组件 (LiquidMeshBackground)
 * 绘制包含 4 个平滑漂浮弥散光斑 (Soft Sky Blue, Soft Ice Cyan, Soft Electric Blue, Soft Pastel Turquoise) 的全屏 Canvas 动态极光流体背景。
 */
@Composable
fun LiquidMeshBackground(
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit
) {
    val infiniteTransition = rememberInfiniteTransition(label = "LiquidMeshTransition")
    val rawTime by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = (2f * Math.PI).toFloat(),
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 16000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "LiquidMeshTime"
    )

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color(0xFFF8FAFC))
    ) {
        Image(
            painter = painterResource(id = R.drawable.bg_custom_ocean),
            contentDescription = null,
            contentScale = ContentScale.Crop,
            modifier = Modifier.fillMaxSize()
        )
        Canvas(modifier = Modifier.fillMaxSize()) {
            val w = size.width
            val h = size.height
            val time = rawTime

            // Blob 1: Soft Sky Blue
            val b1X = w * (0.30f + 0.18f * sin(time))
            val b1Y = h * (0.25f + 0.14f * cos(time * 0.8f))
            val b1Radius = w * 0.70f
            drawCircle(
                brush = Brush.radialGradient(
                    colors = Blob1Colors,
                    center = Offset(b1X, b1Y),
                    radius = b1Radius
                ),
                center = Offset(b1X, b1Y),
                radius = b1Radius
            )

            // Blob 2: Soft Ice Cyan
            val b2X = w * (0.75f + 0.16f * cos(time * 0.9f))
            val b2Y = h * (0.38f + 0.16f * sin(time * 1.1f))
            val b2Radius = w * 0.60f
            drawCircle(
                brush = Brush.radialGradient(
                    colors = Blob2Colors,
                    center = Offset(b2X, b2Y),
                    radius = b2Radius
                ),
                center = Offset(b2X, b2Y),
                radius = b2Radius
            )

            // Blob 3: Soft Electric Blue
            val b3X = w * (0.38f + 0.16f * cos(time * 1.2f))
            val b3Y = h * (0.72f + 0.15f * sin(time * 0.75f))
            val b3Radius = w * 0.65f
            drawCircle(
                brush = Brush.radialGradient(
                    colors = Blob3Colors,
                    center = Offset(b3X, b3Y),
                    radius = b3Radius
                ),
                center = Offset(b3X, b3Y),
                radius = b3Radius
            )

            // Blob 4: Soft Pastel Turquoise
            val b4X = w * (0.82f + 0.14f * sin(time * 0.85f))
            val b4Y = h * (0.80f + 0.16f * cos(time * 1.05f))
            val b4Radius = w * 0.55f
            drawCircle(
                brush = Brush.radialGradient(
                    colors = Blob4Colors,
                    center = Offset(b4X, b4Y),
                    radius = b4Radius
                ),
                center = Offset(b4X, b4Y),
                radius = b4Radius
            )

            // Top-to-bottom light gradient scrim to elevate foreground contrast & depth
            drawRect(
                brush = Brush.verticalGradient(
                    colors = ScrimColors
                )
            )
        }

        content()
    }
}
