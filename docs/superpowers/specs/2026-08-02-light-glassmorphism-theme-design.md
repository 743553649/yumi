# yumi Bridge Android App - iOS 26 真实浅色毛玻璃 (Light Glassmorphism) 重构规格设计书 (v3.1.0)

> **文档日期**：2026-08-02  
> **设计目标**：构建“白色渐变灰”背景与半透明冰粹玻璃 (Translucent Frosted Glassmorphism) 视觉效果，彻底解决纯实色卡片没有毛玻璃透光质感的问题。

---

## 1. 核心视觉设计规范

### 1.1 背景天幕 (White-to-Gray Gradient Backdrop)
- **主背景渐变**：从顶部的纯白 (`#FFFFFF`) 渐变过渡至中部的淡冰灰 (`#F1F5F9`) 与底部的风琴灰 (`#E2E8F0`)。
- **底层弥散彩晕 (Ambient Mesh Glows)**：在白色渐变灰天幕下方放置 4 个色彩鲜明、适度透明的弥散光晕，使浮于其上的半透明玻璃卡片能够产生真正的透光折射质感：
  - 左上角：`#403B82F6` (湛蓝弥散)
  - 右上角：`#358B5CF6` (梦幻紫弥散)
  - 中右侧：`#30F59E0B` (暖阳光橙弥散)
  - 左下角：`#3510B981` (翡翠绿弥散)

### 1.2 浅色毛玻璃卡片 (Frosted Translucent Glass Cards)
- **透明度策略**：摒弃 100% 不透明实色白，采用 **80% ~ 85% 半透明高亮冰白 (`#C8FFFFFF` / `#D8FFFFFF`)**。
- **Fresnel 镜面高光边框**：卡片边缘包含 `1.2dp` 半透明晶莹白外描边 (`#90FFFFFF` / `#FFFFFF`) 与微弱底部投影描边 (`#20000000` / `#40CBD5E1`)，营造立体悬浮感。
- **4 种性能模式彩色半透明玻璃 (Color-Tinted Frosted Glass)**：
  - 🍃 **省电 (`powersave`)**：90% 半透明薄荷绿玻璃 (`#E5DCFCE7`) + 2dp 翡翠绿描边 (`#16A34A`)
  - ⚖️ **均衡 (`balance`)**：90% 半透明冰晶蓝玻璃 (`#E5DBEAFE`) + 2dp 宝石蓝描边 (`#2563EB`)
  - 🚀 **性能 (`performance`)**：90% 半透明日光橙玻璃 (`#E5FFEDD5`) + 2dp 暖橙描边 (`#EA580C`)
  - ⚡ **极速 (`fast`)**：90% 半透明绯红玻璃 (`#E5FEE2E2`) + 2dp 鲜红描边 (`#DC2626`)

### 1.3 终端与 Chip 控件
- **日志终端卡片**：85% 半透明纯白玻璃面板 (`#D8FFFFFF`)，上下渐变遮罩使用白色半透明过渡 (`#F5FFFFFF` $\rightarrow$ `#00FFFFFF`)。
- **Chip 沉浸控件**：未选中态为 60% 凹陷半透明灰白 (`#A0F1F5F9`)，选中态为冰蓝半透明悬浮 (`#D0DBEAFE`) + `#2563EB` 高光。

---

## 2. Token 色值明细 (`colors.xml`)

| Token 名称 | 极简浅色毛玻璃色值 | 描述 |
| :--- | :--- | :--- |
| `ios_bg_dark` | `#FFFFFF` | 天幕底层纯白 |
| `ios_glass_card_bg` | `#C8FFFFFF` | 80% 半透明冰白玻璃卡片底色 |
| `ios_glass_card_bg_selected` | `#E6FFFFFF` | 90% 半透明亮白卡片选态 |
| `ios_glass_stroke_default` | `#90FFFFFF` | 晶莹白镜面高光边框 |
| `ios_glass_stroke_focused` | `#2563EB` | 聚焦高光蓝边框 |
| `ios_glass_input_bg` | `#A0F1F5F9` | 60% 凹陷半透明 Chip 背景 |
| `ios_text_primary` | `#0F172A` | 深邃墨黑主文本 (15.8:1 超高对比度) |
| `ios_text_secondary` | `#475569` | 优雅中灰副文本 |
| `ios_text_muted` | `#64748B` | 分组与辅助文本 |
| `ios_terminal_glass_bg` | `#D8FFFFFF` | 85% 半透明终端面板 |
| `ios_terminal_mask_dark` | `#F5FFFFFF` | 浅白终端上下遮罩 |
| `ios_terminal_mask_trans` | `#00FFFFFF` | 终端遮罩透明端 |
| `ios_terminal_text` | `#0F172A` | 高对比度日志文本 |

---

## 3. 关联修改文件列表

1. `android-app/app/src/main/res/values/colors.xml`
2. `android-app/app/src/main/res/drawable/bg_ios_backdrop.xml`
3. `android-app/app/src/main/res/drawable/bg_ios_glass_card.xml`
4. `android-app/app/src/main/res/drawable/bg_ios_glass_card_fallback.xml`
5. `android-app/app/src/main/res/drawable/bg_ios_glass_input.xml`
6. `android-app/app/src/main/res/drawable/bg_ios_glass_btn.xml`
7. `android-app/app/src/main/res/drawable/bg_ios_mode_powersave.xml`
8. `android-app/app/src/main/res/drawable/bg_ios_mode_balance.xml`
9. `android-app/app/src/main/res/drawable/bg_ios_mode_performance.xml`
10. `android-app/app/src/main/res/drawable/bg_ios_mode_fast.xml`
11. `android-app/app/src/main/res/layout/activity_main.xml`
12. `android-app/app/src/main/java/com/yumi/bridge/MainActivity.java`
