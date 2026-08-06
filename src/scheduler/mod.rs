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

// Scheduler — 核心调度模块。CPU Policy 探测拆分到 policy.rs，
// 主线程编排拆分到 runner.rs；fas/config/scheduler/cpu_load_governor 为既有子模块。

pub mod config;
pub mod cpu_load_governor;
pub mod fas;
pub mod scheduler;

mod policy;
mod runner;

pub use policy::{CpuPolicy, get_cpu_policies};
pub(crate) use policy::{auto_compute_capacity_weights, probe_policy_capacity};
pub use runner::start_scheduler_thread;
