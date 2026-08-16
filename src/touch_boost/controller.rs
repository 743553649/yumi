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

use std::fs;
use std::time::Instant;

use anyhow::Result;
use log::{debug, info};

use crate::i18n::t;
use crate::touch_boost::config::TouchBoostConfig;
use crate::utils::FastWriter;

pub struct TouchBoostController {
    config: TouchBoostConfig,
    cluster_writers: Vec<FastWriter>,
    available_freqs: Vec<Vec<u32>>,
    current_boost_freqs: Vec<u32>,
    boost_until: Instant,
    touch_released_at: Option<Instant>,
    is_boosting: bool,
    disabled: bool,
}

impl TouchBoostController {
    pub fn new(config: TouchBoostConfig) -> Result<Self> {
        let (writers, freq_lists, initial_freqs) = Self::init_cluster_writers(&config);

        info!("{}", t("touch-boost-init"));

        Ok(Self {
            config,
            cluster_writers: writers,
            available_freqs: freq_lists,
            current_boost_freqs: initial_freqs,
            boost_until: Instant::now(),
            touch_released_at: None,
            is_boosting: false,
            disabled: false,
        })
    }

    pub fn disabled() -> Self {
        Self {
            config: TouchBoostConfig::default(),
            cluster_writers: Vec::new(),
            available_freqs: Vec::new(),
            current_boost_freqs: Vec::new(),
            boost_until: Instant::now(),
            touch_released_at: None,
            is_boosting: false,
            disabled: true,
        }
    }

    pub fn on_touch_start(&mut self) {
        if self.disabled {
            return;
        }
        self.is_boosting = true;
        self.touch_released_at = None;
        self.boost_until =
            Instant::now() + std::time::Duration::from_millis(self.config.min_boost_duration_ms);
        self.apply_boost();
        debug!("{}", t("touch-boost-start"));
    }

    pub fn on_touch_end(&mut self) {
        if self.disabled {
            return;
        }
        self.touch_released_at = Some(Instant::now());
        debug!("{}", t("touch-boost-release"));
    }

    pub fn update(&mut self) {
        if self.disabled || !self.is_boosting {
            return;
        }

        if let Some(released_at) = self.touch_released_at {
            let elapsed = released_at.elapsed().as_millis() as u64;
            if elapsed < self.config.release_delay_ms {
                return;
            }
            if Instant::now() < self.boost_until {
                return;
            }

            let decay_factor = self.config.recover_decay;
            let mut all_recovered = true;
            let mut freq_updates: Vec<(usize, u32)> = Vec::new();

            for (i, freq) in self.current_boost_freqs.iter_mut().enumerate() {
                if i >= self.config.boost_freqs.len() {
                    break;
                }
                let target = self.config.boost_freqs[i];
                if target == 0 {
                    continue;
                }

                if *freq > 0 {
                    let raw = (*freq as f32 * (1.0 - decay_factor)) as u32;
                    let new_freq = Self::find_nearest_freq(
                        &self
                            .available_freqs
                            .get(i)
                            .map_or(&[][..], |v| v.as_slice()),
                        raw,
                    );
                    if new_freq <= 100000 {
                        *freq = 0;
                        freq_updates.push((i, 0));
                    } else {
                        *freq = new_freq;
                        freq_updates.push((i, new_freq));
                        all_recovered = false;
                    }
                }
            }

            for (i, freq) in freq_updates {
                self.write_freq(i, freq);
            }

            if all_recovered {
                self.is_boosting = false;
                self.touch_released_at = None;
                debug!("{}", t("touch-boost-recovered"));
            }
        }
    }

    pub fn reload_config(&mut self, config: TouchBoostConfig) {
        self.config = config;
        let (writers, freq_lists, initial_freqs) = Self::init_cluster_writers(&self.config);
        self.cluster_writers = writers;
        self.available_freqs = freq_lists;
        self.current_boost_freqs = initial_freqs;
        info!("{}", t("touch-boost-config-reloaded"));
    }

    fn apply_boost(&mut self) {
        let freqs: Vec<(usize, u32)> = self
            .config
            .boost_freqs
            .iter()
            .enumerate()
            .filter(|&(_, &freq)| freq != 0)
            .map(|(i, &freq)| (i, freq))
            .collect();

        for (i, target_freq) in freqs {
            if i < self.cluster_writers.len() {
                self.current_boost_freqs[i] = target_freq;
                self.write_freq(i, target_freq);
            }
        }
    }

    fn write_freq(&mut self, cluster_idx: usize, freq: u32) {
        if let Some(writer) = self.cluster_writers.get_mut(cluster_idx) {
            writer.write_value_force(freq);
        }
    }

    fn init_cluster_writers(
        config: &TouchBoostConfig,
    ) -> (Vec<FastWriter>, Vec<Vec<u32>>, Vec<u32>) {
        let mut writers = Vec::new();
        let mut freq_lists = Vec::new();
        let mut initial_freqs = Vec::new();

        if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu/cpufreq") {
            let mut policies: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map_or(false, |n| n.starts_with("policy"))
                })
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect();
            policies.sort();

            for (i, policy_name) in policies.iter().enumerate() {
                let min_freq_path = format!(
                    "/sys/devices/system/cpu/cpufreq/{}/scaling_min_freq",
                    policy_name
                );
                let writer = FastWriter::new(&min_freq_path);

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

                writers.push(writer);
                freq_lists.push(freqs);
                initial_freqs.push(config.boost_freqs.get(i).copied().unwrap_or(0));
            }
        }

        (writers, freq_lists, initial_freqs)
    }

    fn find_nearest_freq(available: &[u32], target: u32) -> u32 {
        if available.is_empty() {
            return target;
        }
        let idx = available.partition_point(|&f| f < target);
        if idx == 0 {
            available[0]
        } else if idx >= available.len() {
            *available.last().unwrap()
        } else {
            let lo = idx - 1;
            if target - available[lo] <= available[idx] - target {
                available[lo]
            } else {
                available[idx]
            }
        }
    }
}
