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

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GpuConfig {
    #[serde(default = "crate::utils::default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub modes: GpuModeConfigs,
    #[serde(default = "default_keepalive")]
    pub keepalive_interval_s: u64,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            enabled: crate::utils::default_true(),
            modes: GpuModeConfigs::default(),
            keepalive_interval_s: default_keepalive(),
        }
    }
}

fn default_keepalive() -> u64 {
    5
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct GpuModeConfigs {
    #[serde(default)]
    pub powersave: GpuModeConfig,
    #[serde(default)]
    pub balance: GpuModeConfig,
    #[serde(default)]
    pub performance: GpuModeConfig,
    #[serde(default)]
    pub fast: GpuModeConfig,
    #[serde(default)]
    pub doze: GpuModeConfig,
}

impl GpuModeConfigs {
    pub fn get(&self, mode: &str) -> Option<&GpuModeConfig> {
        match mode {
            "powersave" => Some(&self.powersave),
            "balance" => Some(&self.balance),
            "performance" => Some(&self.performance),
            "fast" => Some(&self.fast),
            "doze" => Some(&self.doze),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct GpuModeConfig {
    #[serde(default)]
    pub max_gpuclk: u32, // 0 = auto-select based on mode
    #[serde(default = "default_gov")]
    pub governor: String,
    #[serde(default)]
    pub force_no_nap: u32, // 0 or 1
}

fn default_gov() -> String {
    "msm-adreno-tz".to_string()
}

impl Default for GpuModeConfig {
    fn default() -> Self {
        Self {
            max_gpuclk: 0,
            governor: default_gov(),
            force_no_nap: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_returns_correct_mode() {
        let configs = GpuModeConfigs {
            powersave: GpuModeConfig {
                max_gpuclk: 100,
                governor: "powersave".into(),
                force_no_nap: 1,
            },
            balance: GpuModeConfig {
                max_gpuclk: 200,
                governor: "msm-adreno-tz".into(),
                force_no_nap: 0,
            },
            performance: GpuModeConfig {
                max_gpuclk: 500,
                governor: "performance".into(),
                force_no_nap: 0,
            },
            fast: GpuModeConfig {
                max_gpuclk: 800,
                governor: "fast".into(),
                force_no_nap: 0,
            },
            doze: GpuModeConfig {
                max_gpuclk: 50,
                governor: "powersave".into(),
                force_no_nap: 1,
            },
        };

        assert_eq!(configs.get("powersave").unwrap().max_gpuclk, 100);
        assert_eq!(configs.get("balance").unwrap().max_gpuclk, 200);
        assert_eq!(configs.get("performance").unwrap().max_gpuclk, 500);
        assert_eq!(configs.get("fast").unwrap().max_gpuclk, 800);
        assert_eq!(configs.get("doze").unwrap().max_gpuclk, 50);
        assert!(configs.get("unknown").is_none());
    }

    #[test]
    fn test_default_config() {
        let cfg = GpuModeConfig::default();
        assert_eq!(cfg.max_gpuclk, 0);
        assert_eq!(cfg.governor, "msm-adreno-tz");
        assert_eq!(cfg.force_no_nap, 0);
    }

    #[test]
    fn test_mode_configs_default_get() {
        let configs = GpuModeConfigs::default();
        let balance = configs.get("balance").unwrap();
        assert_eq!(balance.max_gpuclk, 0);
        assert_eq!(balance.governor, "msm-adreno-tz");
    }
}
