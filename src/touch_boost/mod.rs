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

// TouchBoost — 触摸提频。实现拆分到 config.rs / controller.rs / monitor.rs。

mod config;
mod controller;
mod monitor;

pub use config::TouchBoostConfig;
pub use controller::{BoostState, TouchBoostController};
pub use monitor::{TouchListener, start_touch_listener_thread};
