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

use std::time::Instant;
use anyhow::Result;
use log::{info, warn};

use crate::i18n::{t, t_with_args};
use crate::idle_dive::config::IdleDiveConfig;
use crate::idle_dive::latency::LatencyWriter;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IdleDiveState {
    Normal,
    Diving,
    DozeDiving,
}

pub struct IdleDiveController {
    config: IdleDiveConfig,
    state: IdleDiveState,
    latency_writer: LatencyWriter,
    dive_timer: Instant,
    exit_timer: Instant,
    disabled: bool,
}

impl IdleDiveController {
    pub fn new(config: IdleDiveConfig) -> Result<Self> {
        let latency_writer = LatencyWriter::new()?;

        info!("{}", t("idle-dive-init"));

        Ok(Self {
            config,
            state: IdleDiveState::Normal,
            latency_writer,
            dive_timer: Instant::now(),
            exit_timer: Instant::now(),
            disabled: false,
        })
    }

    pub fn disabled() -> Self {
        Self {
            config: IdleDiveConfig::default(),
            state: IdleDiveState::Normal,
            latency_writer: LatencyWriter::disabled(),
            dive_timer: Instant::now(),
            exit_timer: Instant::now(),
            disabled: true,
        }
    }

    pub fn update(&mut self, avg_util: f32) {
        if self.disabled { return; }

        match self.state {
            IdleDiveState::Normal => {
                if avg_util < self.config.dive_threshold {
                    if self.dive_timer.elapsed().as_millis() as u64 >= self.config.dive_delay_ms {
                        self.transition_to(IdleDiveState::Diving);
                    }
                } else {
                    self.dive_timer = Instant::now();
                }
            }
            IdleDiveState::Diving => {
                if avg_util > self.config.exit_threshold {
                    if self.exit_timer.elapsed().as_millis() as u64 >= self.config.exit_delay_ms {
                        self.transition_to(IdleDiveState::Normal);
                    }
                } else {
                    self.exit_timer = Instant::now();
                }
            }
            IdleDiveState::DozeDiving => {}
        }
    }

    pub fn enter_doze(&mut self) {
        if self.disabled { return; }
        self.transition_to(IdleDiveState::DozeDiving);
    }

    pub fn exit_doze(&mut self) {
        if self.disabled { return; }
        if self.state == IdleDiveState::DozeDiving {
            self.transition_to(IdleDiveState::Normal);
        }
    }

    pub fn on_touch_fast_exit(&mut self) {
        if self.disabled { return; }
        if self.state != IdleDiveState::Normal {
            self.transition_to(IdleDiveState::Normal);
        }
    }

    pub fn reload_config(&mut self, mut config: IdleDiveConfig) {
        config.normalize();
        self.config = config;
        info!("{}", t("idle-dive-config-reloaded"));
    }

    fn transition_to(&mut self, new_state: IdleDiveState) {
        if self.state == new_state { return; }

        match new_state {
            IdleDiveState::Normal => {
                info!("{}", t("idle-dive-exit"));
                if let Err(e) = self.latency_writer.set_governor(&self.config.governors.normal) {
                    warn!("{}", t_with_args("idle-dive-set-governor-failed", &fluent_args!("state" => "normal", "error" => e.to_string())));
                }
                if let Err(e) = self.latency_writer.set_latency(self.config.params.normal_latency_us) {
                    warn!("{}", t_with_args("idle-dive-set-latency-failed", &fluent_args!("state" => "normal", "error" => e.to_string())));
                }
                self.dive_timer = Instant::now();
            }
            IdleDiveState::Diving => {
                info!("{}", t("idle-dive-enter"));
                if let Err(e) = self.latency_writer.set_governor(&self.config.governors.diving) {
                    warn!("{}", t_with_args("idle-dive-set-governor-failed", &fluent_args!("state" => "diving", "error" => e.to_string())));
                }
                if let Err(e) = self.latency_writer.set_latency(self.config.params.diving_latency_us) {
                    warn!("{}", t_with_args("idle-dive-set-latency-failed", &fluent_args!("state" => "diving", "error" => e.to_string())));
                }
                self.exit_timer = Instant::now();
            }
            IdleDiveState::DozeDiving => {
                info!("{}", t("idle-dive-enter-dozed"));
                if let Err(e) = self.latency_writer.set_governor(&self.config.governors.doze) {
                    warn!("{}", t_with_args("idle-dive-set-governor-failed", &fluent_args!("state" => "doze", "error" => e.to_string())));
                }
                if let Err(e) = self.latency_writer.set_latency(self.config.params.doze_latency_us) {
                    warn!("{}", t_with_args("idle-dive-set-latency-failed", &fluent_args!("state" => "doze", "error" => e.to_string())));
                }
            }
        }

        self.state = new_state;
    }
}
