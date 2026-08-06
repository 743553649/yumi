/*
 * Copyright (C) 2026 yuki
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ════════════════════════════════════════════════════════════════
//  PID 系数 (60fps 基准值，运行时根据 target_fps 动态缩放)
//
//  kp: 比例增益 — 按 target_fps/60 线性缩放
//  ki: 积分增益 — 按 sqrt(target_fps/60) 缩放（防高刷积分饱和）
//  kd: 微分增益 — 按 (target_fps/60)^0.3 缩放（高刷噪声大）
// ════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PidCoefficients {
    /// 比例增益基准 (60fps)，高刷时自动放大
    #[serde(default = "default_kp")]  pub kp: f32,
    /// 积分增益基准 (60fps)，高刷时缓增
    #[serde(default = "default_ki")]  pub ki: f32,
    /// 微分增益基准 (60fps)，高刷时微增
    #[serde(default = "default_kd")]  pub kd: f32,
}
fn default_kp() -> f32 { 0.050 }
fn default_ki() -> f32 { 0.010 }
fn default_kd() -> f32 { 0.006 }
impl Default for PidCoefficients {
    fn default() -> Self { Self { kp: default_kp(), ki: default_ki(), kd: default_kd() } }
}

// ════════════════════════════════════════════════════════════════
//  Cluster 配置
// ════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterProfile {
    #[serde(default = "default_capacity_weight")]
    pub capacity_weight: f32,
}
fn default_capacity_weight() -> f32 { 1.0 }
impl Default for ClusterProfile {
    fn default() -> Self { Self { capacity_weight: 1.0 } }
}
pub fn default_cluster_profiles() -> Vec<ClusterProfile> {
    vec![
        ClusterProfile { capacity_weight: 1.0 },
        ClusterProfile { capacity_weight: 1.5 },
        ClusterProfile { capacity_weight: 2.5 },
        ClusterProfile { capacity_weight: 3.5 },
    ]
}

// ════════════════════════════════════════════════════════════════
//  Per-App 配置
// ════════════════════════════════════════════════════════════════

/// 每个游戏的配置档案
///
/// 只需要指定 target_fps 数组，
/// 运行时根据实际帧率动态匹配最近的档位。
///
/// YAML 示例:
/// ```yaml
/// per_app_profiles:
///   "com.miHoYo.GenshinImpact":
///     target_fps: [30, 60]
///     fps_margin: 4.0
///
///   "com.tencent.tmgp.sgame":
///     target_fps: [60, 90, 120]
///     fps_margin: 3.0
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PerAppProfile {
    /// 该游戏会渲染到的目标帧率数组，运行时动态匹配
    /// 例如 [30, 60] 表示游戏可能以 30fps 或 60fps 渲染
    #[serde(default)]
    pub target_fps: Option<Vec<f32>>,

    /// 该应用的帧率余量（覆盖全局 fps_margin）
    #[serde(default)]
    pub fps_margin: Option<f32>,
}

// ════════════════════════════════════════════════════════════════
//  FAS Rules 配置
// ════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FasRulesConfig {
    #[serde(default = "default_fps_gears")]       pub fps_gears: Vec<f32>,
    #[serde(default = "default_fps_margin")]       pub fps_margin: f32,
    #[serde(default)]                              pub pid: PidCoefficients,
    #[serde(default = "default_cluster_profiles")] pub cluster_profiles: Vec<ClusterProfile>,
    #[serde(default = "default_auto_capacity_weight")]               pub auto_capacity_weight: bool,

    #[serde(default = "default_perf_floor")]   pub perf_floor: f32,
    #[serde(default = "default_perf_ceil")]    pub perf_ceil: f32,
    #[serde(default = "default_perf_init")]    pub perf_init: f32,
    #[serde(default = "default_perf_cold_boot")]    pub perf_cold_boot: f32,
    #[serde(default = "default_freq_hysteresis")]   pub freq_hysteresis: f32,

    #[serde(default = "default_heavy_frame_threshold_ms")]     pub heavy_frame_threshold_ms: f32,
    #[serde(default = "default_loading_cumulative_ms")]      pub loading_cumulative_ms: f32,
    #[serde(default = "default_loading_normal_tolerance")]     pub loading_normal_tolerance: u32,
    #[serde(default = "default_loading_perf_floor")]      pub loading_perf_floor: f32,
    #[serde(default = "default_loading_perf_ceiling")]      pub loading_perf_ceiling: f32,

    #[serde(default = "default_post_loading_ignore_frames")]     pub post_loading_ignore_frames: u32,
    #[serde(default = "default_post_loading_perf")]    pub post_loading_perf: f32,
    #[serde(default = "default_post_loading_downgrade_guard")]   pub post_loading_downgrade_guard: u32,

    #[serde(default = "default_upgrade_confirm_frames")]   pub upgrade_confirm_frames: u32,
    #[serde(default = "default_downgrade_confirm_frames")]   pub downgrade_confirm_frames: u32,
    #[serde(default = "default_upgrade_cooldown_after_downgrade")]        pub upgrade_cooldown_after_downgrade: u32,
    #[serde(default = "default_gear_dampen_frames")]       pub gear_dampen_frames: u32,

    #[serde(default = "default_downgrade_boost_perf_inc")]    pub downgrade_boost_perf_inc: f32,
    #[serde(default = "default_downgrade_boost_duration")]    pub downgrade_boost_duration: u32,

    #[serde(default = "default_fast_decay_frame_threshold")]    pub fast_decay_frame_threshold: u32,
    #[serde(default = "default_fast_decay_perf_threshold")]      pub fast_decay_perf_threshold: f32,
    #[serde(default = "default_fast_decay_max_step")]       pub fast_decay_max_step: f32,
    #[serde(default = "default_fast_decay_min_step")]       pub fast_decay_min_step: f32,

    #[serde(default = "default_jank_cooldown_frames")]      pub jank_cooldown_frames: u32,

    #[serde(default = "default_max_inc_damped")]    pub max_inc_damped: f32,
    #[serde(default = "default_max_inc_normal")]    pub max_inc_normal: f32,
    #[serde(default = "default_damped_perf_cap")]   pub damped_perf_cap: f32,

    #[serde(default = "default_app_switch_gap_ms")]    pub app_switch_gap_ms: f32,
    #[serde(default = "default_app_switch_resume_perf")]  pub app_switch_resume_perf: f32,

    #[serde(default = "default_freq_force_reapply_interval")]    pub freq_force_reapply_interval: u32,
    #[serde(default = "default_fixed_max_frame_ms")]    pub fixed_max_frame_ms: f32,
    #[serde(default = "default_cold_boot_ms")]      pub cold_boot_ms: u64,

    #[serde(default = "default_verify_freq_interval_secs")]
    pub verify_freq_interval_secs: u32,

    #[serde(default)]
    pub per_app_profiles: HashMap<String, PerAppProfile>,

    #[serde(default)]
    pub per_app_margins: HashMap<String, f32>,

    /// 温度降频阈值（℃），0 = 禁用
    #[serde(default = "default_core_temp_threshold")]
    pub core_temp_threshold: f64,

    /// 温度降频时的最低 perf
    #[serde(default = "default_core_temp_throttle_perf")]
    pub core_temp_throttle_perf: f32,

    /// CPU 负载辅助：前台线程利用率封顶的除数 (越小越激进)
    #[serde(default = "default_util_cap_divisor")]
    pub util_cap_divisor: f32,
}

pub fn default_fps_gears() -> Vec<f32> { vec![30.0, 60.0, 90.0, 120.0, 144.0] }
pub fn default_fps_margin() -> f32 { 2.0 }
fn default_auto_capacity_weight() -> bool { true }
fn default_perf_floor() -> f32 { 0.22 }
fn default_perf_ceil() -> f32 { 1.0 }
fn default_perf_init() -> f32 { 0.35 }
fn default_perf_cold_boot() -> f32 { 0.85 }
pub fn default_freq_hysteresis() -> f32 { 0.015 }
pub fn default_heavy_frame_threshold_ms() -> f32 { 150.0 }
pub fn default_loading_cumulative_ms() -> f32 { 2500.0 }
fn default_loading_normal_tolerance() -> u32 { 3 }
fn default_loading_perf_floor() -> f32 { 0.60 }
fn default_loading_perf_ceiling() -> f32 { 0.70 }
pub fn default_post_loading_ignore_frames() -> u32 { 5 }
pub fn default_post_loading_perf() -> f32 { 0.50 }
fn default_post_loading_downgrade_guard() -> u32 { 45 }
fn default_upgrade_confirm_frames() -> u32 { 60 }
fn default_downgrade_confirm_frames() -> u32 { 45 }
fn default_upgrade_cooldown_after_downgrade() -> u32 { 90 }
fn default_gear_dampen_frames() -> u32 { 60 }
fn default_downgrade_boost_perf_inc() -> f32 { 0.12 }
fn default_downgrade_boost_duration() -> u32 { 30 }
fn default_fast_decay_frame_threshold() -> u32 { 75 }
fn default_fast_decay_perf_threshold() -> f32 { 0.40 }
fn default_fast_decay_max_step() -> f32 { 0.045 }
fn default_fast_decay_min_step() -> f32 { 0.008 }
fn default_jank_cooldown_frames() -> u32 { 15 }
fn default_max_inc_damped() -> f32 { 0.045 }
fn default_max_inc_normal() -> f32 { 0.075 }
fn default_damped_perf_cap() -> f32 { 0.92 }
fn default_app_switch_gap_ms() -> f32 { 3000.0 }
fn default_app_switch_resume_perf() -> f32 { 0.60 }
fn default_freq_force_reapply_interval() -> u32 { 30 }
fn default_fixed_max_frame_ms() -> f32 { 500.0 }
fn default_cold_boot_ms() -> u64 { 3500 }
fn default_verify_freq_interval_secs() -> u32 { 3 }
fn default_core_temp_threshold() -> f64 { 0.0 }
fn default_core_temp_throttle_perf() -> f32 { 0.70 }
fn default_util_cap_divisor() -> f32 { 0.45 }

impl FasRulesConfig {
    /// 将旧的 per_app_margins 迁移到 per_app_profiles
    pub fn migrate_legacy_margins(&mut self) {
        for (pkg, margin) in self.per_app_margins.drain() {
            self.per_app_profiles
                .entry(pkg)
                .or_default()
                .fps_margin = Some(margin);
        }
    }
}

impl Default for FasRulesConfig {
    fn default() -> Self {
        Self {
            fps_gears: default_fps_gears(), fps_margin: default_fps_margin(),
            pid: PidCoefficients::default(),
            cluster_profiles: default_cluster_profiles(),
            auto_capacity_weight: default_auto_capacity_weight(),
            perf_floor: default_perf_floor(), perf_ceil: default_perf_ceil(),
            perf_init: default_perf_init(), perf_cold_boot: default_perf_cold_boot(),
            freq_hysteresis: default_freq_hysteresis(),
            heavy_frame_threshold_ms: default_heavy_frame_threshold_ms(),
            loading_cumulative_ms: default_loading_cumulative_ms(),
            loading_normal_tolerance: default_loading_normal_tolerance(),
            loading_perf_floor: default_loading_perf_floor(), loading_perf_ceiling: default_loading_perf_ceiling(),
            post_loading_ignore_frames: default_post_loading_ignore_frames(),
            post_loading_perf: default_post_loading_perf(),
            post_loading_downgrade_guard: default_post_loading_downgrade_guard(),
            upgrade_confirm_frames: default_upgrade_confirm_frames(),
            downgrade_confirm_frames: default_downgrade_confirm_frames(),
            upgrade_cooldown_after_downgrade: default_upgrade_cooldown_after_downgrade(),
            gear_dampen_frames: default_gear_dampen_frames(),
            downgrade_boost_perf_inc: default_downgrade_boost_perf_inc(),
            downgrade_boost_duration: default_downgrade_boost_duration(),
            fast_decay_frame_threshold: default_fast_decay_frame_threshold(),
            fast_decay_perf_threshold: default_fast_decay_perf_threshold(),
            fast_decay_max_step: default_fast_decay_max_step(), fast_decay_min_step: default_fast_decay_min_step(),
            jank_cooldown_frames: default_jank_cooldown_frames(),
            max_inc_damped: default_max_inc_damped(), max_inc_normal: default_max_inc_normal(),
            damped_perf_cap: default_damped_perf_cap(),
            app_switch_gap_ms: default_app_switch_gap_ms(), app_switch_resume_perf: default_app_switch_resume_perf(),
            freq_force_reapply_interval: default_freq_force_reapply_interval(),
            fixed_max_frame_ms: default_fixed_max_frame_ms(), cold_boot_ms: default_cold_boot_ms(),
            verify_freq_interval_secs: default_verify_freq_interval_secs(),
            per_app_profiles: HashMap::new(),
            per_app_margins: HashMap::new(),
            core_temp_threshold: default_core_temp_threshold(),
            core_temp_throttle_perf: default_core_temp_throttle_perf(),
            util_cap_divisor: default_util_cap_divisor(),
        }
    }
}
