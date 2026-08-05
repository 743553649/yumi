package com.yumi.bridge.ui.compose

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxWidth
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
import com.kyant.backdrop.drawBackdrop
import com.kyant.backdrop.effects.blur

/**
 * Liquid Glassmorphism Container
 */
private val highlightColors = listOf(
    Color(0xF2FFFFFF), // Top 95% crisp white reflection
    Color(0x40FFFFFF), // Middle 25% subtle white
    Color(0x6038BDF8)  // Bottom soft liquid cyan edge
)

private val glassColors = listOf(
    Color(0x2EFFFFFF), // 18% white fallback
    Color(0x12FFFFFF)  // 7% white fallback
)

@Composable
private fun rememberHighlightBorder() = remember {
    BorderStroke(
        width = 1.2.dp,
        brush = Brush.linearGradient(
            colors = highlightColors,
            start = Offset(0f, 0f),
            end = Offset(Float.POSITIVE_INFINITY, Float.POSITIVE_INFINITY)
        )
    )
}

@Composable
private fun rememberGlassBrush() = remember {
    Brush.verticalGradient(colors = glassColors)
}

@Composable
fun GlassBackdropWrapper(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(24.dp),
    blurRadius: Dp = 12.dp,
    content: @Composable BoxScope.() -> Unit
) {
    val highlightBorder = rememberHighlightBorder()
    val glassBrush = rememberGlassBrush()
    val backdrop = sharedBackdropState.value

    val outerModifier = modifier
        .shadow(elevation = 6.dp, shape = shape, spotColor = Color(0x1A0284C7))
        .clip(shape)
        .border(highlightBorder, shape)
        .let {
            if (backdrop != null) {
                it.drawBackdrop(
                    backdrop = backdrop,
                    shape = { shape },
                    effects = { blur(blurRadius.toPx()) }
                )
            } else it
        }

    Box(modifier = outerModifier) {
        if (backdrop == null) {
            Box(
                modifier = Modifier
                    .matchParentSize()
                    .blur(radius = blurRadius)
                    .background(brush = glassBrush)
            )
        } else {
            Box(
                modifier = Modifier
                    .matchParentSize()
                    .background(Color(0x14FFFFFF)) // Uniform 8% translucent white overlay
            )
        }
        Box(
            modifier = Modifier.fillMaxWidth(),
            content = content
        )
    }
}
