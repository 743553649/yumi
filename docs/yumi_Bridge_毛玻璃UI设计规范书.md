# yumi Bridge Android App - 真实毛玻璃 (Glassmorphism) UI 设计规范书

> **文档版本**：v5.1.0 (Light Glass Micro-Gradient Architecture)  
> **适用平台**：Android 8.0 ~ Android 14+ (API 26 ~ API 34+ Target SDK 34)  
> **核心目标**：提供可落地的真实浅色毛玻璃（银白/浅灰蓝微渐变背景 + 85% 冰白半透明透光卡片 + 硬件加速抗锯齿 + 减弱动画无障碍）完整设计与工程实现指南。

---

## 目录
1. [毛玻璃核心视觉模型与图层堆叠](#1-毛玻璃核心视觉模型与图层堆叠)
2. [浅色微渐变背景设计规范](#2-浅色微渐变背景设计规范)
3. [毛玻璃卡片材质规范与双态 Token 映射](#3-毛玻璃卡片材质规范与双态-token-映射)
4. [选中状态高光与模式选态规范](#4-选中状态高光与模式选态规范)
5. [降级与兼容规范](#5-降级与兼容规范)
6. [实现架构指南 (GlassCardView & 硬件抗锯齿)](#6-实现架构指南)
7. [无障碍、触控热区与减弱动画规范](#7-无障碍触控热区与减弱动画规范)
8. [验证标准](#8-验证标准)

---

## 1. 毛玻璃核心视觉模型与图层堆叠

### 1.1 视觉构成与物理管道
真实的浅色毛玻璃质感采用 **浅色微渐变天幕 $\rightarrow$ 半透明冰白填充叠加 $\rightarrow$ 渐变高光描边 $\rightarrow$ 锐利高对比度文字** 的 5 层物理图层堆叠模型：

```
┌─────────────────────────────────────────────────────────┐ ◄── Layer 4: 前景内容层 (Child Views / Text / Icons) 
│   [ 守护进程已连接 ]   [ 🍃 省电模式 ]   [ 运行日志 ]    │      (100% 像素级清晰，绝对禁止应用 Blur)
├─────────────────────────────────────────────────────────┤ ◄── Layer 3: Fresnel 顶部渐变高光描边 
│   Top Stroke: #FFFFFF ──────► Bottom Stroke: #80FFFFFF   │      (渐变光折射描边)
├─────────────────────────────────────────────────────────┤ ◄── Layer 2: 半透明 Tint 叠加填充层 
│   #D9FFFFFF (亮调 85% 冰白半透明混色)                    │      (色彩基调调和)
├─────────────────────────────────────────────────────────┤ ◄── Layer 1: 实时背景截取与高斯模糊层 (Blur Layer)
│   Blur Radius: 25.0f (RenderEffect / RenderScript)      │      (背景微渐变透光散射)
├─────────────────────────────────────────────────────────┤ ◄── Layer 0: 银白/浅灰蓝微渐变背景天幕 (Backdrop)
│   bg_ios_backdrop (#F8FAFC ──────► #E2E8F0 简洁微渐变)   │      (全屏延伸穿透系统栏)
└─────────────────────────────────────────────────────────┘
```

### 1.2 图层叠加顺序表 (Layer Stack Order)

| 图层顺序 | 名称 | 作用与物理原理 | 关键参数/技术 |
| :---: | :--- | :--- | :--- |
| **Layer 0** | **Backdrop Surface** | 银白/浅灰蓝微渐变天幕，提供简洁干净基底 | 铺满全屏 (`Cutout Short Edges`) |
| **Layer 1** | **Blur Texture** | 从卡片区域实时截取背景并应用高斯模糊 | API 31+: `25.0f` 高斯模糊半径 |
| **Layer 2** | **Tint Overlay** | 半透明冰白混色叠加，提供卡片透明材质感 | 亮调 `#D9FFFFFF` (85% 冰白半透明) |
| **Layer 3** | **Fresnel Edge** | 模拟物理玻璃边缘光线的全内反射 | 顶部 `#FFFFFF` 向下 `#80FFFFFF` 渐变 |
| **Layer 4** | **Foreground Content** | 控件文本、图标与交互元素 | **独立渲染，100% 像素级清晰** |

---

## 2. 浅色微渐变背景设计规范

### 2.1 沉浸与全屏延伸
`bg_ios_backdrop` 彻底消除系统导航栏与状态栏黑色内边距，直接延伸至硬件物理边缘：
1. **Window 标志设置**：`statusBarColor = Color.TRANSPARENT`，`navigationBarColor = Color.TRANSPARENT`。
2. **布局延伸**：`setDecorFitsSystemWindows(false)`。
3. **刘海/挖孔屏穿透**：`layoutInDisplayCutoutMode = LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES`。

### 2.2 浅灰蓝/银白微渐变天幕参数表 (`bg_ios_backdrop.xml`)

移除椭圆形光斑与复杂径向渐变，采用极简、干净的 135° 浅灰蓝/银白微线性渐变：

```xml
<?xml version="1.0" encoding="utf-8"?>
<layer-list xmlns:android="http://schemas.android.com/apk/res/android">
    <!-- 浅灰蓝/银白微渐变底色 (Monochrome Light Ice Gray/Blue Backdrop) -->
    <item>
        <shape android:shape="rectangle">
            <gradient
                android:type="linear"
                android:angle="135"
                android:startColor="#F8FAFC"
                android:centerColor="#F1F5F9"
                android:endColor="#E2E8F0" />
        </shape>
    </item>
</layer-list>
```

---

## 3. 毛玻璃卡片材质规范与双态 Token 映射

### 3.1 物理参数规格
* **曲面圆角**：`22dp` 连续圆角曲率（`corners android:radius="22dp"`）。
* **模糊层 (Blur Layer)**：截取卡片映射区域背景，应用 **`25.0f`** 高斯模糊半径。
* **Fresnel 渐变描边 (Fresnel Rim Light)**：
  * 顶部描边：起点 `#FFFFFF`（晶莹纯白），向卡片底部渐变至 `#80FFFFFF`（半透明白）。
* **立影物理参数 (Drop Shadow)**：
  * 偏移：$X = 0\text{dp}$, $Y = 4\text{dp}$，模糊半径：$Blur = 12\text{dp}$，阴影颜色：`#12000000`（7% 极淡柔影）。

### 3.2 动态双态色彩 Token 映射表

| Token 名称 | 浅色冰粹模式 (Light Glass) | 暗色玻璃模式 (Dark Glass) | 对应渲染层 |
| :--- | :--- | :--- | :--- |
| `ios_bg_dark` | `#F8FAFC` | `#0F172A` | 天幕底层背景色 |
| `ios_glass_card_bg` | `#D9FFFFFF` (85% 亮白半透明) | `#1A1E293B` (10% 暗混色) | Layer 2 填充叠加层 |
| `ios_glass_stroke_default` | `#FFFFFF` $\rightarrow$ `#80FFFFFF` | `#55FFFFFF` $\rightarrow$ `#00FFFFFF` | Layer 3 Fresnel 渐变描边 |
| `ios_glass_input_bg` | `#40F1F5F9` | `#334155` | 凹陷 Chip 徽章背景 |
| `ios_text_primary` | `#0F172A` (15.8:1 AAA级) | `#FFFFFF` (100%) | 核心文字与标题 |
| `ios_text_secondary` | `#475569` (7.4:1 AAA级) | `#CBD5E1` | 副标题与描述 |

---

## 4. 选中状态高光与模式选态规范

### 4.1 模式外发光与脉冲 (Mode Glow & Elevation)
当卡片被激活选中时，外发光色彩与描边实时跟随该模式的主配色：
- 🍃 **省电 (`powersave`)**：翡翠绿描边与发光 (`#16A34A`)，卡片背景 `#F0F0FDF4`
- ⚖️ **均衡 (`balance`)**：宝石蓝描边与发光 (`#2563EB`)，卡片背景 `#F0EFF6FF`
- 🚀 **性能 (`performance`)**：炽热橙描边与发光 (`#EA580C`)，卡片背景 `#F0FFF7ED`
- ⚡ **极速 (`fast`)**：魅红描边与发光 (`#DC2626`)，卡片背景 `#F0FEF2F2`

---

## 5. 降级与兼容规范

针对不同 Android API 版本提供三级平滑降级渲染架构：

| 兼容层级 | Android API 版本 | 技术实现方案 | 降级视觉策略 |
| :---: | :--- | :--- | :--- |
| **Tier 1 (原生)** | **API 31+ (Android 12+)** | `RenderEffect.createBlurEffect(25f, 25f, Shader.TileMode.CLAMP)` | 100% 真实实时背景区域透光模糊 |
| **Tier 2 (RS 缓存)** | **API 29 - 30 (Android 10 - 11)** | `RenderScript` / `ScriptIntrinsicBlur` | 捕获底图 $\rightarrow$ 缩小 4 倍 $\rightarrow$ 25f 模糊 $\rightarrow$ 缓存 |
| **Tier 3 (保质降级)** | **API < 29 (Android 8 - 9)** | 实色混合 + 噪点纹理 Overlay | 混色 `#D9FFFFFF`（85% 不透明）+ 保留渐变高光 |

---

## 6. 实现架构指南 (GlassCardView & 硬件抗锯齿)

### 6.1 关键硬件级抗锯齿与 ViewOutlineProvider 优化
优先使用 `ViewOutlineProvider` 配合 `setClipToOutline(true)` 进行硬件级圆角裁剪：

```java
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

public class GlassCardView extends FrameLayout {

    private final Paint tintPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint strokePaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF rectF = new RectF();
    private float cornerRadius;

    private int customTint = 0xD9FFFFFF; // 85% 冰白半透明混色
    private int customStrokeStart = 0xFFFFFFFF;
    private int customStrokeEnd = 0x80FFFFFF;

    public GlassCardView(Context context, AttributeSet attrs) {
        super(context, attrs);
        init(context);
    }

    private void init(Context context) {
        setWillNotDraw(false);
        float density = context.getResources().getDisplayMetrics().density;
        cornerRadius = density * 22f; // 22dp

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
        strokePaint.setStrokeWidth(density * 1.2f);
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
        super.onSizeChanged(w, h, oldw, oldh);
        float density = getResources().getDisplayMetrics().density;
        float halfStroke = density * 0.6f;
        rectF.set(halfStroke, halfStroke, w - halfStroke, h - halfStroke);

        LinearGradient strokeShader = new LinearGradient(
                0, 0, 0, h,
                customStrokeStart, customStrokeEnd,
                Shader.TileMode.CLAMP
        );
        strokePaint.setShader(strokeShader);
    }

    @Override
    protected void dispatchDraw(Canvas canvas) {
        // 1. 绘制 Layer 2 半透明 Tint 叠加填充层 (85% 冰白)
        canvas.drawRoundRect(rectF, cornerRadius, cornerRadius, tintPaint);

        // 2. 绘制 Layer 3 Fresnel 顶部渐变高光描边
        canvas.drawRoundRect(rectF, cornerRadius, cornerRadius, strokePaint);

        // 3. 绘制 Layer 4 前景内容 (文字/图标 100% 像素级清晰)
        super.dispatchDraw(canvas);
    }
}
```

---

## 7. 无障碍、触控热区与减弱动画规范

### 7.1 触控热区扩充 (Touch Target Minimum $\ge 48\text{dp}$)
日志过滤 Chip 标签与清空日志等交互控件在 XML 布局中通过 `android:minHeight="48dp"` 与 `android:paddingVertical="8dp"` 扩展物理触控范围 $\ge 48\text{dp} \times 48\text{dp}$。

### 7.2 减弱动态效果支持 (Reduced Motion Compliance)
当用户开启“减弱动态效果 (Reduced Motion)”时禁用 Overshoot 弹簧与呼吸脉冲动画。

---

## 8. 验证标准

- [x] **极简浅色天幕**：天幕采用 `#F8FAFC` $\rightarrow$ `#E2E8F0` 简洁浅灰蓝微渐变，彻底移除椭圆光斑。
- [x] **透明冰粹质感**：毛玻璃卡片维持 85% 冰白半透明透光感，底层微渐变穿透。
- [x] **文字极致锐利**：TextView 深色 Slate 文字保持 100% 像素级清晰（对比度 15.8:1 AAA级）。
- [x] **硬件级无锯齿**：圆角边缘使用 `ViewOutlineProvider` 剪裁。
- [x] **触控热区达标**：物理 Touch Target 均 $\ge 48\text{dp}$。

---

*设计与架构规范生成日期：2026-08-02*  
*系统版本：yumi Bridge v5.1.0 Light Glass Architecture*
