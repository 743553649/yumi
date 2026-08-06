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
//  配置结构 (对应 config/touch_boost.yaml)
// ════════════════════════════════════════════════════════════════

use serde::Deserialize;

/// TouchBoost 配置
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct TouchBoostConfig {
    /// 是否启用 TouchBoost
    pub enabled: bool,
    /// 各集群的 boost 目标频率 (kHz)，按 policy id 索引
    /// 例如: [2500000, 0, 2000000] 表示 Policy 0 → 2.5GHz，Policy 2 → 2.0GHz
    pub boost_freqs: Vec<u32>,
    /// 松手后恢复延迟 (ms)，防止快速点击时频繁切换
    pub release_delay_ms: u64,
    /// 恢复阶段的衰减步长 (每次 tick 降低当前 boost 频率的比例)
    pub recover_decay: f32,
    /// 最小 boost 持续时间 (ms)，防止误触
    pub min_boost_duration_ms: u64,
    /// 触摸设备路径，留空则自动检测
    pub input_device: String,
}

impl Default for TouchBoostConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            boost_freqs: vec![2500000, 0, 2000000],
            release_delay_ms: 100,
            recover_decay: 0.15,
            min_boost_duration_ms: 50,
            input_device: String::new(),
        }
    }
}

// ════════════════════════════════════════════════════════════════
//  单元测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证配置解析：扁平顶层结构
    #[test]
    fn test_config_parses_flat_top_level() {
        let yaml = r#"
enabled: true
boost_freqs:
  - 2500000
  - 0
  - 2000000
release_delay_ms: 100
recover_decay: 0.15
min_boost_duration_ms: 50
input_device: ""
"#;
        let cfg: TouchBoostConfig = serde_yaml::from_str(yaml).expect("解析失败");
        assert!(cfg.enabled);
        assert_eq!(cfg.boost_freqs.len(), 3);
        assert_eq!(cfg.boost_freqs[0], 2500000);
        assert_eq!(cfg.boost_freqs[2], 2000000);
        assert_eq!(cfg.release_delay_ms, 100);
    }

    /// 验证配置缺失字段时使用默认值
    #[test]
    fn test_config_defaults_for_missing_fields() {
        let yaml = "enabled: false\n";
        let cfg: TouchBoostConfig = serde_yaml::from_str(yaml).expect("解析失败");
        assert!(!cfg.enabled);
        assert_eq!(cfg.boost_freqs, vec![2500000, 0, 2000000]);
        assert_eq!(cfg.release_delay_ms, 100);
        assert_eq!(cfg.recover_decay, 0.15);
    }
}
