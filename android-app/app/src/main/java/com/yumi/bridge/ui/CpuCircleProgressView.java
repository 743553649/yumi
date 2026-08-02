package com.yumi.bridge.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RectF;
import android.util.AttributeSet;
import android.view.View;

public class CpuCircleProgressView extends View {

    private final Paint bgPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint progressPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint textPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF oval = new RectF();

    private int progress = 0; // 0 到 100
    private float strokeWidth = 4.5f; // dp

    public CpuCircleProgressView(Context context) {
        super(context);
        init(context);
    }

    public CpuCircleProgressView(Context context, AttributeSet attrs) {
        super(context, attrs);
        init(context);
    }

    public CpuCircleProgressView(Context context, AttributeSet attrs, int defStyleAttr) {
        super(context, attrs, defStyleAttr);
        init(context);
    }

    private void init(Context context) {
        float density = context.getResources().getDisplayMetrics().density;
        float sw = strokeWidth * density;

        bgPaint.setStyle(Paint.Style.STROKE);
        bgPaint.setStrokeWidth(sw);
        bgPaint.setColor(0x253B82F6); // 25% 冰蓝背景环轨

        progressPaint.setStyle(Paint.Style.STROKE);
        progressPaint.setStrokeWidth(sw);
        progressPaint.setStrokeCap(Paint.Cap.ROUND);
        progressPaint.setColor(0xFF3B82F6); // 主调 iOS 宝石蓝

        textPaint.setColor(0xFF0F172A); // 深 Slate 高对比度文字
        textPaint.setTextSize(10.5f * density);
        textPaint.setTextAlign(Paint.Align.CENTER);
        textPaint.setFakeBoldText(true);
    }

    public void setProgress(int progress) {
        int clamped = Math.max(0, Math.min(100, progress));
        if (this.progress != clamped) {
            this.progress = clamped;
            
            // 高负载变色规则
            if (clamped > 80) {
                progressPaint.setColor(0xFFEF4444); // 极速红
            } else if (clamped > 60) {
                progressPaint.setColor(0xFFF59E0B); // 性能橙
            } else if (clamped > 30) {
                progressPaint.setColor(0xFF3B82F6); // 均衡蓝
            } else {
                progressPaint.setColor(0xFF10B981); // 省电绿
            }
            invalidate();
        }
    }

    public int getProgress() {
        return this.progress;
    }

    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);

        int width = getWidth();
        int height = getHeight();
        if (width <= 0 || height <= 0) return;

        float density = getResources().getDisplayMetrics().density;
        float sw = strokeWidth * density;
        float halfStroke = sw / 2f;

        oval.set(halfStroke + getPaddingLeft(),
                 halfStroke + getPaddingTop(),
                 width - halfStroke - getPaddingRight(),
                 height - halfStroke - getPaddingBottom());

        // 1. 底轨 360 度圆环
        canvas.drawArc(oval, 0, 360, false, bgPaint);

        // 2. 从顶部 (-90°) 顺时针绘制当前进度弧度
        float sweepAngle = (progress / 100f) * 360f;
        if (sweepAngle > 0) {
            canvas.drawArc(oval, -90, sweepAngle, false, progressPaint);
        }

        // 3. 居中绘制 0-100% 文本
        String text = progress + "%";
        float textY = (height / 2f) - ((textPaint.descent() + textPaint.ascent()) / 2f);
        canvas.drawText(text, width / 2f, textY, textPaint);
    }
}
