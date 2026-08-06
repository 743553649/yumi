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

// ════════════════════════════════════════════════════════════════
//  配置结构 (对应 config/idle_dive.yaml)
// ════════════════════════════════════════════════════════════════

use serde::Deserialize;

/// CPU 静止下潜配置
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct IdleDiveConfig {
    /// 是否启用
    pub enabled: bool,
    /// 触发下潜的负载阈值 (平均负载低于此值触发)
    pub dive_threshold: f32,
    /// 退出下潜的负载阈值 (平均负载高于此值退出)
    pub exit_threshold: f32,
    /// 下潜延迟 (ms)，负载持续低于阈值多久后触发
    pub dive_delay_ms: u64,
    /// 退出延迟 (ms)，负载持续高于阈值多久后退出
    pub exit_delay_ms: u64,
    /// 各状态下的 cpuidle governor
    pub governors: IdleDiveGovernors,
    /// 各状态下的 idle latency 参数
    pub params: IdleDiveParams,
}

impl Default for IdleDiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dive_threshold: 0.15,
            exit_threshold: 0.25,
            dive_delay_ms: 300,
            exit_delay_ms: 50,
            governors: IdleDiveGovernors::default(),
            params: IdleDiveParams::default(),
        }
    }
}

/// 各状态下的 cpuidle governor
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct IdleDiveGovernors {
    /// 正常状态的 governor
    pub normal: String,
    /// 下潜状态的 governor
    pub diving: String,
    /// 息屏状态的 governor
    pub doze: String,
}

impl Default for IdleDiveGovernors {
    fn default() -> Self {
        // governor 默认值全部相同 ("menu")，主要靠 latency_us 参数调节深度
        Self {
            normal: "menu".to_string(),
            diving: "menu".to_string(),
            doze: "menu".to_string(),
        }
    }
}

/// 各状态下的 idle latency 参数 (μs)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct IdleDiveParams {
    /// 正常状态允许的 idle 延迟 (μs)
    pub normal_latency_us: u32,
    /// 下潜状态允许的 idle 延迟 (μs)
    pub diving_latency_us: u32,
    /// 息屏状态允许的 idle 延迟 (μs)
    pub doze_latency_us: u32,
}

impl Default for IdleDiveParams {
    fn default() -> Self {
        Self {
            normal_latency_us: 100,
            diving_latency_us: 500,
            doze_latency_us: 1000,
        }
    }
}

// ════════════════════════════════════════════════════════════════
//  单元测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证配置解析：顶层键为扁平结构 (无 idle_dive 包装键)
    /// 回归测试：与 cpuset.yaml 包装键问题同源，包装键会导致配置永远加载默认值
    #[test]
    fn test_config_parses_flat_top_level() {
        let yaml = r#"
enabled: true
dive_threshold: 0.12
exit_threshold: 0.35
dive_delay_ms: 1500
exit_delay_ms: 300
governors:
  normal: "menu"
  diving: "ladder"
  doze: "powersave"
params:
  normal_latency_us: 80
  diving_latency_us: 600
  doze_latency_us: 1200
"#;
        let cfg: IdleDiveConfig = serde_yaml::from_str(yaml).expect("解析失败");
        assert!(cfg.enabled);
        assert_eq!(cfg.dive_threshold, 0.12);
        assert_eq!(cfg.dive_delay_ms, 1500);
        assert_eq!(cfg.governors.diving, "ladder");
        assert_eq!(cfg.params.doze_latency_us, 1200);
    }

    /// 验证配置缺失字段时使用默认值
    #[test]
    fn test_config_defaults_for_missing_fields() {
        let yaml = "enabled: false\n";
        let cfg: IdleDiveConfig = serde_yaml::from_str(yaml).expect("解析失败");
        assert!(!cfg.enabled);
        assert_eq!(cfg.dive_threshold, 0.15);
        assert_eq!(cfg.exit_threshold, 0.25);
        assert_eq!(cfg.dive_delay_ms, 300);
        assert_eq!(cfg.exit_delay_ms, 50);
        assert_eq!(cfg.governors.normal, "menu");
        assert_eq!(cfg.params.normal_latency_us, 100);
    }
}
