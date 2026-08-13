# Task 2 Report：修复 TouchBoost 频率取整

## 1. FastWriter 调查结果

### FastWriter 结构体 (`src/utils.rs:175-255`)
- **功能**：带去重 + unmount 的 sysfs 写入器
- **字段**：`file: Option<File>`, `buf: [u8; 20]`, `path: PathBuf`
- **关键方法**：`write_value_force(u32)` 直接写入值，不校验频率有效性
- **结论**：FastWriter **没有**可用频率列表，只负责写入

### 已有频率查找函数
- `ClusterState::find_nearest_freq(&self, target_ratio: f32) -> u32`（`cpu_load_governor.rs:63`）
- `PolicyController::find_nearest_freq(&self, target_ratio: f32) -> u32`（`fas/policy_controller.rs:70`）
- 两者均基于 **ratio（比例）** 查找，需要 `cached_ratios` 和 `available_freqs` 配合
- **不适合直接复用**，因为 TouchBoost 使用绝对频率值（kHz），不是 ratio

### 可用频率读取方式
```rust
// 从 cpu_load_governor.rs:160-168 已有模式
let freq_path = format!(
    "/sys/devices/system/cpu/cpufreq/policy{}/scaling_available_frequencies", pid);
let mut freqs: Vec<u32> = fs::read_to_string(&freq_path)
    .unwrap_or_default()
    .split_whitespace()
    .filter_map(|s| s.parse().ok())
    .collect();
freqs.sort_unstable();
freqs.dedup();
```

## 2. 选择的修复方案

采用**方案 1**：在 `init_cluster_writers` 中读取每个 policy 的 `scaling_available_frequencies`，存储在控制器中，衰减时用二分查找 snap 到最近有效频率。

**设计决策**：
- 新增 `available_freqs: Vec<Vec<u32>>` 字段存储每个 cluster 的有效频率列表
- 新增 `find_nearest_freq(available: &[u32], target: u32) -> u32` 静态方法
- 使用 `partition_point` 二分查找（与项目内其他 find_nearest_freq 一致的算法模式）
- 空列表时回退到原始值（target），保持鲁棒性
- 保留原有的 `<= 100000` 归零阈值逻辑

## 3. 实现代码

### 新增字段
```rust
pub struct TouchBoostController {
    config: TouchBoostConfig,
    cluster_writers: Vec<FastWriter>,
    available_freqs: Vec<Vec<u32>>,  // 新增
    current_boost_freqs: Vec<u32>,
    // ...
}
```

### 衰减逻辑修改（update 方法）
```rust
let raw = (*freq as f32 * (1.0 - decay_factor)) as u32;
let new_freq = Self::find_nearest_freq(
    &self.available_freqs.get(i).map_or(&[][..], |v| v.as_slice()),
    raw,
);
```

### init_cluster_writers 修改
在创建 writer 的同时，读取 `scaling_available_frequencies`：
```rust
let avail_path = format!(
    "/sys/devices/system/cpu/cpufreq/{}/scaling_available_frequencies",
    policy_name
);
let mut freqs: Vec<u32> = fs::read_to_string(&avail_path)
    .unwrap_or_default()
    .split_whitespace()
    .filter_map(|s| s.parse().ok())
    .collect();
freqs.sort_unstable();
freqs.dedup();
```

### 新增 find_nearest_freq 方法
```rust
fn find_nearest_freq(available: &[u32], target: u32) -> u32 {
    if available.is_empty() { return target; }
    let idx = available.partition_point(|&f| f < target);
    if idx == 0 { available[0] }
    else if idx >= available.len() { *available.last().unwrap() }
    else {
        let lo = idx - 1;
        if target - available[lo] <= available[idx] - target {
            available[lo]
        } else {
            available[idx]
        }
    }
}
```

## 4. 代码差异

```diff
diff --git a/src/touch_boost/controller.rs b/src/touch_boost/controller.rs
index b3a4c5e..b1be17d 100644
--- a/src/touch_boost/controller.rs
+++ b/src/touch_boost/controller.rs
@@ -28,6 +28,7 @@ use crate::utils::FastWriter;
 pub struct TouchBoostController {
     config: TouchBoostConfig,
     cluster_writers: Vec<FastWriter>,
+    available_freqs: Vec<Vec<u32>>,
     current_boost_freqs: Vec<u32>,
     boost_until: Instant,
     touch_released_at: Option<Instant>,
@@ -37,13 +38,14 @@ pub struct TouchBoostController {
 
 impl TouchBoostController {
     pub fn new(config: TouchBoostConfig) -> Result<Self> {
-        let (writers, initial_freqs) = Self::init_cluster_writers(&config);
+        let (writers, freq_lists, initial_freqs) = Self::init_cluster_writers(&config);
 
         info!("{}", t("touch-boost-init"));
 
         Ok(Self {
             config,
             cluster_writers: writers,
+            available_freqs: freq_lists,
             current_boost_freqs: initial_freqs,
             boost_until: Instant::now(),
             touch_released_at: None,
@@ -56,6 +58,7 @@ impl TouchBoostController {
         Self {
             config: TouchBoostConfig::default(),
             cluster_writers: Vec::new(),
+            available_freqs: Vec::new(),
             current_boost_freqs: Vec::new(),
             boost_until: Instant::now(),
             touch_released_at: None,
@@ -97,7 +100,11 @@ impl TouchBoostController {
                 if target == 0 { continue; }
 
                 if *freq > 0 {
-                    let new_freq = (*freq as f32 * (1.0 - decay_factor)) as u32;
+                    let raw = (*freq as f32 * (1.0 - decay_factor)) as u32;
+                    let new_freq = Self::find_nearest_freq(
+                        &self.available_freqs.get(i).map_or(&[][..], |v| v.as_slice()),
+                        raw,
+                    );
                     if new_freq <= 100000 {
                         *freq = 0;
                         freq_updates.push((i, 0));
@@ -123,8 +130,9 @@ impl TouchBoostController {
 
     pub fn reload_config(&mut self, config: TouchBoostConfig) {
         self.config = config;
-        let (writers, initial_freqs) = Self::init_cluster_writers(&self.config);
+        let (writers, freq_lists, initial_freqs) = Self::init_cluster_writers(&self.config);
         self.cluster_writers = writers;
+        self.available_freqs = freq_lists;
         self.current_boost_freqs = initial_freqs;
         info!("{}", t("touch-boost-config-reloaded"));
     }
@@ -149,8 +157,9 @@ impl TouchBoostController {
         }
     }
 
-    fn init_cluster_writers(config: &TouchBoostConfig) -> (Vec<FastWriter>, Vec<u32>) {
+    fn init_cluster_writers(config: &TouchBoostConfig) -> (Vec<FastWriter>, Vec<Vec<u32>>, Vec<u32>) {
         let mut writers = Vec::new();
+        let mut freq_lists = Vec::new();
         let mut initial_freqs = Vec::new();
 
         if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu/cpufreq") {
@@ -169,13 +178,42 @@ impl TouchBoostController {
                     policy_name
                 );
                 let writer = FastWriter::new(&min_freq_path);
+
+                let avail_path = format!(
+                    "/sys/devices/system/cpu/cpufreq/{}/scaling_available_frequencies",
+                    policy_name
+                );
+                let mut freqs: Vec<u32> = fs::read_to_string(&avail_path)
+                    .unwrap_or_default()
+                    .split_whitespace()
+                    .filter_map(|s| s.parse().ok())
+                    .collect();
+                freqs.sort_unstable();
+                freqs.dedup();
+
                 writers.push(writer);
+                freq_lists.push(freqs);
                 initial_freqs.push(
                     config.boost_freqs.get(i).copied().unwrap_or(0)
                 );
             }
         }
 
-        (writers, initial_freqs)
+        (writers, freq_lists, initial_freqs)
+    }
+
+    fn find_nearest_freq(available: &[u32], target: u32) -> u32 {
+        if available.is_empty() { return target; }
+        let idx = available.partition_point(|&f| f < target);
+        if idx == 0 { available[0] }
+        else if idx >= available.len() { *available.last().unwrap() }
+        else {
+            let lo = idx - 1;
+            if target - available[lo] <= available[idx] - target {
+                available[lo]
+            } else {
+                available[idx]
+            }
+        }
     }
 }
```

## 5. 验证结果

- **rustfmt --check**：代码解析成功，仅存在格式化偏好差异（项目既有风格使用单行 if 块）
- **cargo check**：因 Termux 环境权限限制无法执行 build script（`proc-macro2` 构建失败），属于环境问题非代码问题
- **逻辑验证**：
  - `find_nearest_freq` 空列表回退到 target（鲁棒）
  - `partition_point` 二分查找正确处理边界（idx=0 / idx>=len）
  - 原有 `<= 100000` 归零逻辑保持不变
  - `init_cluster_writers` 返回类型从 `(Vec<FastWriter>, Vec<u32>)` 改为 `(Vec<FastWriter>, Vec<Vec<u32>>, Vec<u32>)`，所有调用点已同步更新
