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
pub struct IdleDiveConfig {
    #[serde(default = "crate::utils::default_true")]
    pub enabled: bool,
    #[serde(default = "d_dive_threshold")]
    pub dive_threshold: f32,
    #[serde(default = "d_exit_threshold")]
    pub exit_threshold: f32,
    #[serde(default = "d_dive_delay_ms")]
    pub dive_delay_ms: u64,
    #[serde(default = "d_exit_delay_ms")]
    pub exit_delay_ms: u64,
    #[serde(default = "d_doze_debounce_ms")]
    pub doze_debounce_ms: u64,
    #[serde(default)]
    pub governors: IdleDiveGovernors,
    #[serde(default)]
    pub params: IdleDiveParams,
}

fn d_dive_threshold() -> f32 {
    0.12
}
fn d_exit_threshold() -> f32 {
    0.18
}
fn d_dive_delay_ms() -> u64 {
    500
}
fn d_exit_delay_ms() -> u64 {
    500
}
fn d_doze_debounce_ms() -> u64 {
    500
}

#[derive(Debug, Deserialize, Clone)]
pub struct IdleDiveGovernors {
    pub normal: String,
    pub diving: String,
    pub doze: String,
}

impl Default for IdleDiveGovernors {
    fn default() -> Self {
        Self {
            normal: "menu".to_string(),
            diving: "menu".to_string(),
            doze: "menu".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct IdleDiveParams {
    pub normal_latency_us: i32,
    pub diving_latency_us: i32,
    pub doze_latency_us: i32,
}

impl Default for IdleDiveParams {
    fn default() -> Self {
        Self {
            normal_latency_us: 100,
            diving_latency_us: 800,
            doze_latency_us: 1500,
        }
    }
}

impl Default for IdleDiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dive_threshold: d_dive_threshold(),
            exit_threshold: d_exit_threshold(),
            dive_delay_ms: d_dive_delay_ms(),
            exit_delay_ms: d_exit_delay_ms(),
            doze_debounce_ms: d_doze_debounce_ms(),
            governors: IdleDiveGovernors::default(),
            params: IdleDiveParams::default(),
        }
    }
}

impl IdleDiveConfig {
    pub fn normalize(&mut self) {
        if !self.dive_threshold.is_finite() {
            self.dive_threshold = d_dive_threshold();
        }
        if !self.exit_threshold.is_finite() {
            self.exit_threshold = d_exit_threshold();
        }
        self.dive_threshold = self.dive_threshold.clamp(0.0, 1.0);
        self.exit_threshold = self.exit_threshold.clamp(0.0, 1.0);
        if self.exit_threshold <= self.dive_threshold {
            self.exit_threshold = (self.dive_threshold + 0.05).min(1.0);
        }
        if self.dive_delay_ms == 0 {
            self.dive_delay_ms = d_dive_delay_ms();
        }
        if self.exit_delay_ms == 0 {
            self.exit_delay_ms = d_exit_delay_ms();
        }
        if self.doze_debounce_ms == 0 {
            self.doze_debounce_ms = d_doze_debounce_ms();
        }
    }
}
