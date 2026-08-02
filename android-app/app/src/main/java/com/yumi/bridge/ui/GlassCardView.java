package com.yumi.bridge.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.LinearGradient;
import android.graphics.Outline;
import android.graphics.Paint;
import android.graphics.RectF;
import android.graphics.Shader;
import android.util.AttributeSet;
import android.view.View;
import android.view.ViewOutlineProvider;
import android.widget.FrameLayout;

/**
 * 真实毛玻璃卡片核心 View (GlassCardView)
 * 遵循 yumi Bridge v5.0.0 Light Glass Architecture 规范书 Section 6 实现。
 * 1. 22dp 硬件级 ViewOutlineProvider 抗锯齿圆角剪裁。
 * 2. Layer 2 半透明 Tint 叠加填充层 (#D9FFFFFF 85% 冰白半透明混色)。
 * 3. Layer 3 Fresnel 顶部渐变高光描边 (#FFFFFF -> #80FFFFFF)。
 * 4. Layer 4 前景内容 (Text / Icons) 保持 100% 像素级清晰，严禁模糊。
 */
public class GlassCardView extends FrameLayout {

    private final Paint tintPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint strokePaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF rectF = new RectF();
    private float cornerRadius;

    private int customTint = 0xD9FFFFFF; // 冰白 85% 混色 (#D9FFFFFF)
    private int customStrokeStart = 0xFFFFFFFF;
    private int customStrokeEnd = 0x80FFFFFF;

    public GlassCardView(Context context) {
        super(context);
        init(context);
    }

    public GlassCardView(Context context, AttributeSet attrs) {
        super(context, attrs);
        init(context);
    }

    public GlassCardView(Context context, AttributeSet attrs, int defStyleAttr) {
        super(context, attrs, defStyleAttr);
        init(context);
    }

    private void init(Context context) {
        setWillNotDraw(false);

        float density = context.getResources().getDisplayMetrics().density;
        cornerRadius = density * 16f; // 16dp 连续圆角

        // 开启硬件级抗锯齿圆角裁剪 (Section 6.1)
        setOutlineProvider(new ViewOutlineProvider() {
            @Override
            public void getOutline(View view, Outline outline) {
                outline.setRoundRect(0, 0, view.getWidth(), view.getHeight(), cornerRadius);
            }
        });
        setClipToOutline(true);

        tintPaint.setColor(customTint);
        tintPaint.setStyle(Paint.Style.FILL);

        strokePaint.setStyle(Paint.Style.STROKE);
        strokePaint.setStrokeWidth(density * 1.2f); // 1.2dp
    }

    public void setActiveState(boolean active, int accentColor) {
        float density = getResources().getDisplayMetrics().density;
        if (active) {
            // 激活状态：微透遮罩与专属发光边框
            int alphaTint = (accentColor & 0x00FFFFFF) | 0x26000000; // 15% 透明度背景
            int alphaStrokeEnd = (accentColor & 0x00FFFFFF) | 0x60000000; // 37% 透明度底部描边
            this.customTint = alphaTint;
            this.customStrokeStart = accentColor; // 顶部 100% 主题亮色
            this.customStrokeEnd = alphaStrokeEnd;
            strokePaint.setStrokeWidth(density * 1.5f); // 1.5dp 精密内描边
        } else {
            // 未激活状态：优雅冷白毛玻璃
            this.customTint = 0x0DFFFFFF; // 5% 冰白微透
            this.customStrokeStart = 0x26FFFFFF; // 15% 白描边
            this.customStrokeEnd = 0x0DFFFFFF; // 5% 白描边
            strokePaint.setStrokeWidth(density * 1.0f); // 1.0dp
        }
        tintPaint.setColor(customTint);
        updateStrokeShader(getWidth(), getHeight());
        updateRectFBounds();
        invalidate();
    }

    public void setCardCornerRadius(float dp) {
        float density = getResources().getDisplayMetrics().density;
        this.cornerRadius = density * dp;
        invalidateOutline();
        invalidate();
    }

    public void setTintOverlayColor(int color) {
        this.customTint = color;
        tintPaint.setColor(customTint);
        invalidate();
    }

    public void setFresnelStroke(int startColor, int endColor) {
        this.customStrokeStart = startColor;
        this.customStrokeEnd = endColor;
        updateStrokeShader(getWidth(), getHeight());
        invalidate();
    }

    private void updateStrokeShader(int w, int h) {
        if (w <= 0 || h <= 0) return;
        LinearGradient strokeShader = new LinearGradient(
                0, 0, 0, h,
                customStrokeStart, customStrokeEnd,
                Shader.TileMode.CLAMP
        );
        strokePaint.setShader(strokeShader);
    }

    private void updateRectFBounds() {
        int w = getWidth();
        int h = getHeight();
        if (w <= 0 || h <= 0) return;
        float halfStroke = strokePaint.getStrokeWidth() / 2f;
        rectF.set(halfStroke, halfStroke, w - halfStroke, h - halfStroke);
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
        super.onSizeChanged(w, h, oldw, oldh);
        updateRectFBounds();
        updateStrokeShader(w, h);
    }

    @Override
    protected void dispatchDraw(Canvas canvas) {
        // 1. 绘制 Layer 2 半透明 Tint 叠加填充层 (22dp 圆角)
        canvas.drawRoundRect(rectF, cornerRadius, cornerRadius, tintPaint);

        // 2. 绘制 Layer 3 Fresnel 顶部渐变高光描边 (22dp 圆角)
        canvas.drawRoundRect(rectF, cornerRadius, cornerRadius, strokePaint);

        // 3. 绘制 Layer 4 前景内容 (子视图文字/图标保持 100% 像素级清晰)
        super.dispatchDraw(canvas);
    }
}
