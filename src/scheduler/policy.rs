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
//  CPU Policy 探测 — 读取 sysfs 中的 cpufreq 策略簇信息与 capacity 权重
// ════════════════════════════════════════════════════════════════

use std::fs;

/// CPU 频率策略簇信息
#[derive(Debug, Clone)]
pub struct CpuPolicy {
    pub id: i32,
    pub cpus: Vec<i32>,
    /// boost 频率列表（单位 kHz），有的簇没有此文件则为空
    pub boost_frequencies: Vec<u32>,
}

// 动态获取系统中实际可用的 CPU Policy，并读取 boost 频率
pub fn get_cpu_policies() -> Vec<CpuPolicy> {
    let mut policies = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu/cpufreq") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("policy") {
                    if let Ok(pid) = name["policy".len()..].parse::<i32>() {
                        let cpus = read_related_cpus(pid);
                        let boost_freqs = read_boost_frequencies(pid);
                        policies.push(CpuPolicy {
                            id: pid,
                            cpus,
                            boost_frequencies: boost_freqs,
                        });
                    }
                }
            }
        }
    }
    policies.sort_unstable_by_key(|p| p.id);
    policies
}

fn read_related_cpus(pid: i32) -> Vec<i32> {
    let path = format!(
        "/sys/devices/system/cpu/cpufreq/policy{}/related_cpus",
        pid
    );
    std::fs::read_to_string(&path)
        .or_else(|_| {
            std::fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/affected_cpus",
                pid
            ))
        })
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn read_boost_frequencies(pid: i32) -> Vec<u32> {
    let path = format!(
        "/sys/devices/system/cpu/cpufreq/policy{}/scaling_boost_frequencies",
        pid
    );
    std::fs::read_to_string(&path)
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// 通过 sysfs 探测指定 policy 的 capacity 值
pub(crate) fn probe_policy_capacity(policy_id: i32) -> Option<u32> {
    let related_str = fs::read_to_string(
        format!("/sys/devices/system/cpu/cpufreq/policy{}/related_cpus", policy_id))
        .or_else(|_| fs::read_to_string(
            format!("/sys/devices/system/cpu/cpufreq/policy{}/affected_cpus", policy_id)))
        .ok()?;
    let first_cpu: u32 = related_str.split_whitespace().next()?.parse().ok()?;
    fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/cpu_capacity", first_cpu))
        .ok()?.trim().parse::<u32>().ok()
}

/// 根据 CPU capacity 自动计算每个 cluster 的权重
pub(crate) fn auto_compute_capacity_weights(policies: &[CpuPolicy]) -> Option<Vec<(i32, f32)>> {
    let caps: Vec<(i32, u32)> = policies.iter()
        .filter(|p| p.id != -1)
        .filter_map(|p| probe_policy_capacity(p.id).map(|c| (p.id, c)))
        .collect();
    if caps.is_empty() || caps.iter().any(|&(_, c)| c == 0) { return None; }
    // 此处已由上文 is_empty / 零值守卫保证非空，unwrap_or(1) 仅作除零兜底
    let min_cap = caps.iter().map(|&(_, c)| c).min().unwrap_or(1) as f32;
    Some(caps.iter().map(|&(pid, cap)| {
        let r = cap as f32 / min_cap;
        (pid, if r <= 1.01 { 1.0 } else { 1.0 + (r - 1.0).sqrt() })
    }).collect())
}
