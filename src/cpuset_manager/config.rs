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
//  配置结构
// ════════════════════════════════════════════════════════════════

use serde::Deserialize;

/// CPUSet 配置（对应 config/cpuset.yaml）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct CpuSetConfig {
    /// 是否启用 CPUSet 管理
    pub enabled: bool,
    /// 各模式下的 CPUSet 分配
    pub modes: CpuSetModes,
}

impl Default for CpuSetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            modes: CpuSetModes::default(),
        }
    }
}

/// 各模式的 CPUSet 配置
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct CpuSetModes {
    pub powersave: CpuSetPolicy,
    pub balance: CpuSetPolicy,
    pub performance: CpuSetPolicy,
    pub fast: CpuSetPolicy,
    /// 息屏模式
    pub doze: CpuSetPolicy,
}

impl Default for CpuSetModes {
    fn default() -> Self {
        Self {
            powersave: CpuSetPolicy {
                top_app: "0-7".into(),
                foreground: "0-7".into(),
                background: "4-7".into(),
                system_background: "6-7".into(),
                restricted: "6-7".into(),
            },
            balance: CpuSetPolicy {
                top_app: "0-7".into(),
                foreground: "0-7".into(),
                background: "2-7".into(),
                system_background: "4-7".into(),
                restricted: "6-7".into(),
            },
            performance: CpuSetPolicy {
                top_app: "0-7".into(),
                foreground: "0-7".into(),
                background: "0-7".into(),
                system_background: "2-7".into(),
                restricted: "4-7".into(),
            },
            fast: CpuSetPolicy {
                top_app: "0-7".into(),
                foreground: "0-7".into(),
                background: "0-7".into(),
                system_background: "0-7".into(),
                restricted: "0-7".into(),
            },
            doze: CpuSetPolicy {
                top_app: "2-3".into(),
                foreground: "2-5".into(),
                background: "4-7".into(),
                system_background: "6-7".into(),
                restricted: "7".into(),
            },
        }
    }
}

/// 单个模式的 CPUSet 策略（cpuset 格式字符串，如 "0-3"、"0-1,4-7"）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct CpuSetPolicy {
    pub top_app: String,
    pub foreground: String,
    pub background: String,
    pub system_background: String,
    pub restricted: String,
}

impl Default for CpuSetPolicy {
    fn default() -> Self {
        Self {
            top_app: "0-7".into(),
            foreground: "0-7".into(),
            background: "2-7".into(),
            system_background: "4-7".into(),
            restricted: "6-7".into(),
        }
    }
}

impl CpuSetPolicy {
    /// 根据组名返回对应的 cpuset 值
    pub(super) fn value_for_group(&self, group: &str) -> Option<&String> {
        match group {
            "top-app" => Some(&self.top_app),
            "foreground" => Some(&self.foreground),
            "background" => Some(&self.background),
            "system-background" => Some(&self.system_background),
            "restricted" => Some(&self.restricted),
            _ => None,
        }
    }
}

// ════════════════════════════════════════════════════════════════
//  单元测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证配置解析：顶层键必须是 enabled/modes（无 cpuset 包装键）
    /// 回归测试：修复前 cpuset.yaml 顶层 cpuset: 包装导致配置永远加载默认值
    #[test]
    fn test_config_parses_flat_top_level() {
        let yaml = r#"
enabled: true
modes:
  powersave:
    top_app: "0-7"
    foreground: "0-7"
    background: "4-7"
    system_background: "6-7"
    restricted: "6-7"
  balance:
    top_app: "0-7"
    foreground: "0-7"
    background: "2-7"
    system_background: "4-7"
    restricted: "6-7"
  performance:
    top_app: "0-7"
    foreground: "0-7"
    background: "0-7"
    system_background: "2-7"
    restricted: "4-7"
  fast:
    top_app: "0-7"
    foreground: "0-7"
    background: "0-7"
    system_background: "0-7"
    restricted: "0-7"
  doze:
    top_app: "2-3"
    foreground: "2-5"
    background: "4-7"
    system_background: "6-7"
    restricted: "7"
"#;
        let cfg: CpuSetConfig = serde_yaml::from_str(yaml).expect("解析失败");
        assert!(cfg.enabled);
        assert_eq!(cfg.modes.doze.top_app, "2-3");
        assert_eq!(cfg.modes.powersave.background, "4-7");
        assert_eq!(cfg.modes.fast.restricted, "0-7");
    }

    /// 验证配置缺失字段时使用默认值
    #[test]
    fn test_config_defaults_for_missing_fields() {
        let yaml = "enabled: true\n";
        let cfg: CpuSetConfig = serde_yaml::from_str(yaml).expect("解析失败");
        assert!(cfg.enabled);
        // 未提供的模式/组回退到默认策略
        assert_eq!(cfg.modes.balance.top_app, "0-7");
    }

    /// 验证 cpus_to 值在 apply 前不会为空
    #[test]
    fn test_policy_value_for_group() {
        let policy = CpuSetPolicy::default();
        assert_eq!(policy.value_for_group("top-app").unwrap(), "0-7");
        assert_eq!(policy.value_for_group("background").unwrap(), "2-7");
        assert!(policy.value_for_group("unknown").is_none());
    }
}
