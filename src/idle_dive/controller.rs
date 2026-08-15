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
use crate::fluent_args;
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
    last_doze_exit: Instant,
    log_cooldown: u32,
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
            last_doze_exit: Instant::now(),
            log_cooldown: 0,
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
            last_doze_exit: Instant::now(),
            log_cooldown: 0,
            disabled: true,
        }
    }

    pub fn update(&mut self, avg_util: f32) {
        if self.disabled { return; }

        if self.log_cooldown > 0 {
            self.log_cooldown -= 1;
        }

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
        if self.last_doze_exit.elapsed().as_millis() < 500 {
            return;
        }
        self.transition_to(IdleDiveState::DozeDiving);
    }

    pub fn exit_doze(&mut self) {
        if self.disabled { return; }
        if self.state == IdleDiveState::DozeDiving {
            self.transition_to(IdleDiveState::Normal);
            self.last_doze_exit = Instant::now();
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

        let (governor, latency_us) = match new_state {
            IdleDiveState::Normal => (&self.config.governors.normal, self.config.params.normal_latency_us),
            IdleDiveState::Diving => (&self.config.governors.diving, self.config.params.diving_latency_us),
            IdleDiveState::DozeDiving => (&self.config.governors.doze, self.config.params.doze_latency_us),
        };

        if self.log_cooldown == 0 {
            match new_state {
                IdleDiveState::Normal => info!("{}", t("idle-dive-exit")),
                IdleDiveState::Diving => info!("{}", t("idle-dive-enter")),
                IdleDiveState::DozeDiving => info!("{}", t("idle-dive-enter-dozed")),
            }
            self.log_cooldown = 5;
        }

        if let Err(e) = self.latency_writer.set_governor(governor) {
            warn!("{}", t_with_args("idle-dive-set-governor-failed", &fluent_args!("state" => format!("{:?}", new_state).to_lowercase(), "error" => e.to_string())));
        }
        if let Err(e) = self.latency_writer.set_latency(latency_us) {
            warn!("{}", t_with_args("idle-dive-set-latency-failed", &fluent_args!("state" => format!("{:?}", new_state).to_lowercase(), "error" => e.to_string())));
        }

        if new_state == IdleDiveState::Normal {
            self.dive_timer = Instant::now();
        } else if new_state == IdleDiveState::Diving {
            self.exit_timer = Instant::now();
        }

        self.state = new_state;
    }
}
