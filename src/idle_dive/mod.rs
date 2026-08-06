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

// CPU 静止下潜 (Idle Dive) — 主动让 CPU 进入更深的 C-state。实现拆分到 config.rs / latency.rs / controller.rs。

mod config;
mod controller;
mod latency;

pub use config::{IdleDiveConfig, IdleDiveGovernors, IdleDiveParams};
pub use controller::{DiveState, IdleDiveController};
pub use latency::LatencyWriter;
