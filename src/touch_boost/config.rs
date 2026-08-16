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

#[derive(Debug, Deserialize, Clone)]
pub struct TouchBoostConfig {
    #[serde(default = "crate::utils::default_true")]
    pub enabled: bool,
    #[serde(default = "d_boost_freqs")]
    pub boost_freqs: Vec<u32>,
    #[serde(default = "d_release_delay_ms")]
    pub release_delay_ms: u64,
    #[serde(default = "d_recover_decay")]
    pub recover_decay: f32,
    #[serde(default = "d_min_boost_duration_ms")]
    pub min_boost_duration_ms: u64,
    #[serde(default)]
    pub input_device: String,
}

fn d_boost_freqs() -> Vec<u32> {
    vec![2500000, 0, 2000000]
}
fn d_release_delay_ms() -> u64 {
    100
}
fn d_recover_decay() -> f32 {
    0.15
}
fn d_min_boost_duration_ms() -> u64 {
    50
}

impl Default for TouchBoostConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            boost_freqs: d_boost_freqs(),
            release_delay_ms: d_release_delay_ms(),
            recover_decay: d_recover_decay(),
            min_boost_duration_ms: d_min_boost_duration_ms(),
            input_device: String::new(),
        }
    }
}

impl TouchBoostConfig {
    pub fn normalize(&mut self) {
        if self.boost_freqs.is_empty() {
            self.boost_freqs = d_boost_freqs();
        }
        if self.release_delay_ms == 0 {
            self.release_delay_ms = d_release_delay_ms();
        }
        if !self.recover_decay.is_finite() {
            self.recover_decay = d_recover_decay();
        }
        self.recover_decay = self.recover_decay.clamp(0.01, 1.0);
        if self.min_boost_duration_ms == 0 {
            self.min_boost_duration_ms = d_min_boost_duration_ms();
        }
    }
}
