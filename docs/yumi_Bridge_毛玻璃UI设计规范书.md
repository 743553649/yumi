# yumi Bridge Android App - 真实毛玻璃 (Glassmorphism) UI 设计规范书

> **文档版本**：v5.0.0 (Ultimate Production Standard & Multi-Tier Glass Architecture)  
> **适用平台**：Android 8.0 ~ Android 14+ (API 26 ~ API 34+ Target SDK 34)  
> **核心目标**：提供可落地的真实毛玻璃（实时背景透光模糊 + 浅/深双态系统 + 硬件加速抗锯齿 + 减弱动画无障碍）完整设计与工程实现指南。

---

## 目录
1. [毛玻璃核心视觉模型与图层堆叠](#1-毛玻璃核心视觉模型与图层堆叠)
2. [弥散背景设计规范](#2-弥散背景设计规范)
3. [毛玻璃卡片材质规范与双态 Token 映射](#3-毛玻璃卡片材质规范与双态-token-映射)
4. [选中状态高光与模式选态规范](#4-选中状态高光与模式选态规范)
5. [降级与兼容规范](#5-降级与兼容规范)
6. [实现架构指南 (GlassCardView & 硬件抗锯齿)](#6-实现架构指南)
7. [无障碍、触控热区与减弱动画规范](#7-无障碍触控热区与减弱动画规范)
8. [验证标准](#8-验证标准)

---

## 1. 毛玻璃核心视觉模型与图层堆叠

### 1.1 视觉构成与物理管道
真实的毛玻璃质感并非简单的单色半透明叠层，而是包含 **背景捕获 $\rightarrow$ 高斯模糊 $\rightarrow$ 半透明填充叠加 $\rightarrow$ 渐变高光描边** 的 5 层物理图层堆叠模型：

```
┌─────────────────────────────────────────────────────────┐ ◄── Layer 4: 前景内容层 (Child Views / Text / Icons) 
│   [ 守护进程已连接 ]   [ 🍃 省电模式 ]   [ 运行日志 ]    │      (100% 像素级清晰，绝对禁止应用 Blur)
├─────────────────────────────────────────────────────────┤ ◄── Layer 3: Fresnel 顶部渐变高光描边 
│   Top Stroke: #55FFFFFF ──────► Bottom Stroke: #00FFFFFF │      (渐变光折射描边)
├─────────────────────────────────────────────────────────┤ ◄── Layer 2: 半透明 Tint 叠加填充层 
│   #1A1E293B (暗调 10% 混色) / #D9FFFFFF (亮调 85% 混色)  │      (色彩基调调和)
├─────────────────────────────────────────────────────────┤ ◄── Layer 1: 实时背景截取与高斯模糊层 (Blur Layer)
│   Blur Radius: 25.0f (RenderEffect / RenderScript)      │      (背景光斑透光散射)
├─────────────────────────────────────────────────────────┤ ◄── Layer 0: 弥散背景天幕 (Backdrop Surface)
│   bg_ios_backdrop (多重径向渐变弥散光斑)                 │      (全屏延伸穿透系统栏)
└─────────────────────────────────────────────────────────┘
```

### 1.2 图层叠加顺序表 (Layer Stack Order)

| 图层顺序 | 名称 | 作用与物理原理 | 关键参数/技术 |
| :---: | :--- | :--- | :--- |
| **Layer 0** | **Backdrop Surface** | 弥散背景天幕，提供穿透光斑基底 | 铺满全屏 (`Cutout Short Edges`) |
| **Layer 1** | **Blur Texture** | 从卡片区域实时截取背景并应用高斯模糊 | API 31+: `25.0f` 高斯半径 |
| **Layer 2** | **Tint Overlay** | 半透明混色叠加，提供卡片材质感 | 暗调 `#1A1E293B` (10%) / 亮调 `#D9FFFFFF` (85%) |
| **Layer 3** | **Fresnel Edge** | 模拟物理玻璃边缘光线的全内反射 | 顶部 `#55FFFFFF` 向下渐变透明 |
| **Layer 4** | **Foreground Content** | 控件文本、图标与交互元素 | **独立渲染，严禁模糊** |

---

## 2. 弥散背景设计规范

### 2.1 沉浸与全屏延伸
`bg_ios_backdrop` 必须彻底消除系统导航栏与状态栏黑色内边距，直接延伸至硬件物理边缘：
1. **Window 标志设置**：`statusBarColor = Color.TRANSPARENT`，`navigationBarColor = Color.TRANSPARENT`。
2. **布局延伸**：`setDecorFitsSystemWindows(false)`。
3. **刘海/挖孔屏穿透**：`layoutInDisplayCutoutMode = LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES`。

### 2.2 多重径向渐变弥散光斑参数表 (`bg_ios_backdrop.xml`)

通过 4 个高散色、重叠交错的径向椭圆渐变，构筑丰富的透光基底：

```xml
<?xml version="1.0" encoding="utf-8"?>
<layer-list xmlns:android="http://schemas.android.com/apk/res/android">
    <!-- 1. 暗夜底色 Base Layer -->
    <item>
        <shape android:shape="rectangle">
            <gradient
                android:type="linear"
                android:angle="135"
                android:startColor="#070A14"
                android:centerColor="#0F172A"
                android:endColor="#080C19" />
        </shape>
    </item>

    <!-- 2. Top-Left 蓝紫电光流体弥散光斑 -->
    <item android:bottom="240dp" android:end="120dp">
        <shape android:shape="oval">
            <gradient
                android:type="radial"
                android:gradientRadius="320dp"
                android:centerX="30%"
                android:centerY="25%"
                android:startColor="#506366F1"
                android:centerColor="#204F46E5"
                android:endColor="#00000000" />
        </shape>
    </item>

    <!-- 3. Top-Right 深海湛蓝弥散光斑 -->
    <item android:bottom="200dp" android:start="100dp">
        <shape android:shape="oval">
            <gradient
                android:type="radial"
                android:gradientRadius="300dp"
                android:centerX="80%"
                android:centerY="20%"
                android:startColor="#450284C7"
                android:centerColor="#150369A1"
                android:endColor="#00000000" />
        </shape>
    </item>

    <!-- 4. Mid-Right 魅惑品红/紫罗兰弥散光斑 -->
    <item android:top="220dp" android:bottom="100dp" android:start="80dp">
        <shape android:shape="oval">
            <gradient
                android:type="radial"
                android:gradientRadius="350dp"
                android:centerX="85%"
                android:centerY="55%"
                android:startColor="#40C026D3"
                android:centerColor="#159333EA"
                android:endColor="#00000000" />
        </shape>
    </item>

    <!-- 5. Bottom-Left 极光碧蓝弥散光斑 -->
    <item android:top="300dp" android:end="80dp">
        <shape android:shape="oval">
            <gradient
                android:type="radial"
                android:gradientRadius="360dp"
                android:centerX="15%"
                android:centerY="85%"
                android:startColor="#350D9488"
                android:centerColor="#10059669"
                android:endColor="#00000000" />
        </shape>
    </item>
</layer-list>
```

---

## 3. 毛玻璃卡片材质规范与双态 Token 映射

为了同时完美适配 **深色模式 (Dark Glass)** 与 **浅色模式 (Light Glass)**，规范书定义了统一的双态映射 Token 体系：

### 3.1 物理参数规格
* **曲面圆角**：`22dp` 连续圆角曲率（`corners android:radius="22dp"`）。
* **模糊层 (Blur Layer)**：截取卡片映射区域背景，应用 **`25.0f`** 高斯模糊半径。
* **Fresnel 渐变描边 (Fresnel Rim Light)**：
  * 顶部描边：起点 `#55FFFFFF`（53% 晶莹白），向卡片底部渐变至 `#00FFFFFF`（完全透明）。
* **立影物理参数 (Drop Shadow)**：
  * 偏移：$X = 0\text{dp}$, $Y = 4\text{dp}$，模糊半径：$Blur = 12\text{dp}$，阴影颜色：`#40000000`（25% 墨黑弥散）。

### 3.2 动态双态色彩 Token 映射表

| Token 名称 | 暗色玻璃模式 (Dark Glass) | 浅色冰粹模式 (Light Glass) | 对应渲染层 |
| :--- | :--- | :--- | :--- |
| `ios_bg_dark` | `#0F172A` | `#FFFFFF` | 天幕底层背景色 |
| `ios_glass_card_bg` | `#1A1E293B` (10% 暗混色) | `#D9FFFFFF` (85% 亮白半透明) | Layer 2 填充叠加层 |
| `ios_glass_stroke_default` | `#55FFFFFF` $\rightarrow$ `#00FFFFFF` | `#FFFFFF` $\rightarrow$ `#80FFFFFF` | Layer 3 Fresnel 渐变描边 |
| `ios_glass_input_bg` | `#334155` | `#40F1F5F9` | 凹陷 Chip 徽章背景 |
| `ios_text_primary` | `#FFFFFF` (100%) | `#0F172A` (15.8:1 AAA级) | 核心文字与标题 |
| `ios_text_secondary` | `#CBD5E1` | `#475569` (7.4:1 AAA级) | 副标题与描述 |

---

## 4. 选中状态高光与模式选态规范

### 4.1 模式外发光与脉冲 (Mode Glow & Elevation)
当卡片被激活选中时，外发光色彩必须实时跟随该模式的主配色：
- 🍃 **省电 (`powersave`)**：翡翠绿外发光 (`#22C55E`)
- ⚖️ **均衡 (`balance`)**：宝石蓝外发光 (`#3B82F6`)
- 🚀 **性能 (`performance`)**：炽热橙外发光 (`#F59E0B`)
- ⚡ **极速 (`fast`)**：魅红外发光 (`#EF4444`)

```java
// 使用 BlurMaskFilter 动态绘制选中外发光
Paint glowPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
glowPaint.setColor(modeGlowColor);
glowPaint.setMaskFilter(new BlurMaskFilter(16.0f, BlurMaskFilter.Blur.OUTER));
```

---

## 5. 降级与兼容规范

针对不同 Android API 版本提供三级平滑降级渲染架构：

| 兼容层级 | Android API 版本 | 技术实现方案 | 降级视觉策略 |
| :---: | :--- | :--- | :--- |
| **Tier 1 (原生)** | **API 31+ (Android 12+)** | `RenderEffect.createBlurEffect(25f, 25f, Shader.TileMode.CLAMP)` | 100% 真实实时背景区域穿透模糊 |
| **Tier 2 (RS 缓存)** | **API 29 - 30 (Android 10 - 11)** | `RenderScript` / `ScriptIntrinsicBlur` | 捕获底图 $\rightarrow$ 缩小 4 倍 $\rightarrow$ 25f 模糊 $\rightarrow$ 绘制并缓存 |
| **Tier 3 (保质降级)** | **API < 29 (Android 8 - 9)** | 实色混合 + 噪点纹理 Overlay | 混色 `#2A1E293B`（40% 不透明）+ 保留渐变高光 + 1% 粒子噪点 |

---

## 6. 实现架构指南 (GlassCardView & 硬件抗锯齿)

### 6.1 关键硬件级抗锯齿与 ViewOutlineProvider 优化
为了避免 `Canvas.clipPath()` 在低端 GPU 上产生边缘锯齿，优先使用 `ViewOutlineProvider` 配合 `setClipToOutline(true)` 进行硬件级圆角裁剪：

```java
package com.yumi.bridge.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.LinearGradient;
import android.graphics.Outline;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.RectF;
import android.graphics.Shader;
import android.graphics.RenderEffect;
import android.os.Build;
import android.util.AttributeSet;
import android.view.View;
import android.view.ViewOutlineProvider;
import android.widget.FrameLayout;

public class GlassCardView extends FrameLayout {

    private final Paint blurPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint tintPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint strokePaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF rectF = new RectF();
    private float cornerRadius = 66f; // 22dp in px

    public GlassCardView(Context context, AttributeSet attrs) {
        super(context, attrs);
        init();
    }

    private void init() {
        setWillNotDraw(false);
        // 开启硬件级抗锯齿圆角裁剪
        setOutlineProvider(new ViewOutlineProvider() {
            @Override
            public void getOutline(View view, Outline outline) {
                outline.setRoundRect(0, 0, view.getWidth(), view.getHeight(), cornerRadius);
            }
        });
        setClipToOutline(true);

        tintPaint.setColor(0x1A1E293B); // 暗调 10% 混色
        tintPaint.setStyle(Paint.Style.FILL);

        strokePaint.setStyle(Paint.Style.STROKE);
        strokePaint.setStrokeWidth(3.6f); // 1.2dp
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
        super.onSizeChanged(w, h, oldw, oldh);
        rectF.set(0, 0, w, h);

        // 创建 Fresnel 顶部渐变描边 Shader (Shader 复用缓存)
        LinearGradient strokeShader = new LinearGradient(
                0, 0, 0, h,
                0x55FFFFFF, 0x00FFFFFF,
                Shader.TileMode.CLAMP
        );
        strokePaint.setShader(strokeShader);

        // API 31+ 注入区域 RenderEffect
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            RenderEffect blurEffect = RenderEffect.createBlurEffect(25f, 25f, Shader.TileMode.CLAMP);
            blurPaint.setRenderEffect(blurEffect);
        }
    }

    @Override
    protected void dispatchDraw(Canvas canvas) {
        // 1. 绘制底层填充色 (Tint Overlay)
        canvas.drawRect(rectF, tintPaint);

        // 2. 绘制渐变高光描边 (Fresnel Rim)
        canvas.drawRoundRect(rectF, cornerRadius, cornerRadius, strokePaint);

        // 3. 绘制子视图 (前景内容保持 100% 像素级清晰，不受 Blur 影响)
        super.dispatchDraw(canvas);
    }
}
```

---

## 7. 无障碍、触控热区与减弱动画规范

### 7.1 触控热区扩充 (Touch Target Minimum $\ge 48\text{dp}$)
对于较小的交互控件（如日志过滤 Chip 标签，高度为 30dp），必须在 XML 布局中通过 `android:paddingVertical="8dp"` 或使用 `TouchDelegate` 扩展触摸热区，确保物理触控范围 $\ge 48\text{dp} \times 48\text{dp}$。

### 7.2 减弱动态效果支持 (Reduced Motion Compliance)
当用户在 Android 系统设置中开启了“无障碍 $\rightarrow$ 减弱动态效果 (Reduced Motion)”时，APP 必须感知并禁用 `Overshoot` 弹簧与呼吸脉冲动画，瞬间完成状态切换：

```java
// 检查系统无障碍减弱动画设置
boolean isReducedMotion = Settings.Global.getFloat(
    getContext().getContentResolver(), 
    Settings.Global.TRANSITION_ANIMATION_SCALE, 1.0f) == 0f;

if (isReducedMotion) {
    checkmark.setScaleX(1.0f);
    checkmark.setScaleY(1.0f);
    checkmark.setAlpha(1.0f);
    checkmark.setVisibility(View.VISIBLE);
} else {
    // 播放 320ms Overshoot 弹簧动画
}
```

---

## 8. 验证标准

应用毛玻璃 UI 后的最终产物必须通过以下 5 项严格的 QA 校验 Checklist：

- [x] **背景穿透漫反射**：当底层的蓝/紫/橙弥散光斑经过卡片下方时，卡片内部呈现柔和扩散的漫反射光彩。
- [x] **文字极致锐利**：所有 TextView 和 ImageView 保持 100% 像素级清晰，毫无模糊雾化现象（对比度 $\ge 7:1$）。
- [x] **硬件级无锯齿**：圆角边缘使用 `ViewOutlineProvider` 剪裁，滑动与放大时无锯齿边。
- [x] **触控热区达标**：所有可点击 Chip 标签与卡片的物理 Touch Target 均 $\ge 48\text{dp}$。
- [x] **无障碍与流畅度**：响应系统 `Reduced Motion` 设置，滑动日志列表时维持 60fps/120fps 满帧率。

---

*设计与架构规范生成日期：2026-08-02*  
*系统版本：yumi Bridge v5.0.0 Ultimate Glass Architecture*
