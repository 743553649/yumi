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

use std::time::{Duration, Instant};

/// GPU health status returned by watchdog checks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuHealth {
    /// GPU is operating normally
    Healthy,
    /// GPU clock appears stalled (same frequency for too long)
    Stalled,
    /// GPU is hung (recovery needed)
    Hung,
}

/// WriteCircuitBreaker: prevents repeated writes to GPU sysfs when
/// they keep failing, with a cooldown period.
#[derive(Debug)]
pub struct WriteCircuitBreaker {
    pub fail_count: u32,
    pub last_fail_time: Instant,
    pub cooldown_until: Option<Instant>,
}

impl WriteCircuitBreaker {
    pub const MAX_FAILURES: u32 = 3;
    pub const COOLDOWN_SECS: u64 = 30;

    pub fn new() -> Self {
        Self {
            fail_count: 0,
            last_fail_time: Instant::now(),
            cooldown_until: None,
        }
    }
}

impl Default for WriteCircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteCircuitBreaker {
    /// Record a write failure. Returns true if the circuit breaker is now tripped.
    pub fn record_failure(&mut self) -> bool {
        self.fail_count += 1;
        self.last_fail_time = Instant::now();
        if self.fail_count >= Self::MAX_FAILURES {
            self.cooldown_until = Some(Instant::now() + Duration::from_secs(Self::COOLDOWN_SECS));
            log::warn!(
                "[GPU] Write circuit breaker tripped after {} failures, cooling down for {}s",
                self.fail_count,
                Self::COOLDOWN_SECS
            );
            true
        } else {
            false
        }
    }

    /// Record a successful write, resetting counters
    pub fn record_success(&mut self) {
        self.fail_count = 0;
        self.cooldown_until = None;
    }

    /// Check if the circuit breaker is currently in cooldown
    pub fn is_cooldown(&self) -> bool {
        self.cooldown_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    /// Try to reset the circuit breaker if cooldown has expired
    pub fn try_reset(&mut self) {
        if let Some(until) = self.cooldown_until
            && Instant::now() >= until
        {
            self.fail_count = 0;
            self.cooldown_until = None;
            log::info!("[GPU] Write circuit breaker reset after cooldown");
        }
    }
}

/// GpuWatchdog: monitors GPU clock frequency for stalls/hangs
#[derive(Debug)]
pub struct GpuWatchdog {
    pub last_gpuclk: u32,
    pub last_check: Instant,
    pub stalled_count: u32,
    pub recovery_attempted: bool,
}

impl GpuWatchdog {
    pub const POLL_INTERVAL: Duration = Duration::from_secs(2);
    pub const STALL_THRESHOLD: u32 = 5; // 5 consecutive checks = ~10s of same freq
    pub const RECOVERY_WRITE_VALUE: u32 = 0;

    pub fn new() -> Self {
        Self {
            last_gpuclk: 0,
            last_check: Instant::now(),
            stalled_count: 0,
            recovery_attempted: false,
        }
    }
}

impl Default for GpuWatchdog {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuWatchdog {
    /// Returns GpuHealth based on stall detection logic.
    pub fn check(&mut self, current_gpuclk: u32) -> GpuHealth {
        let now = Instant::now();
        if now.duration_since(self.last_check) < Self::POLL_INTERVAL {
            return GpuHealth::Healthy;
        }
        self.last_check = now;

        if self.last_gpuclk == current_gpuclk && current_gpuclk > 0 {
            self.stalled_count += 1;
        } else {
            self.stalled_count = 0;
        }
        self.last_gpuclk = current_gpuclk;

        if self.stalled_count >= Self::STALL_THRESHOLD {
            if self.recovery_attempted {
                GpuHealth::Hung
            } else {
                GpuHealth::Stalled
            }
        } else {
            GpuHealth::Healthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_one_failure_does_not_trip() {
        let mut cb = WriteCircuitBreaker::new();
        assert!(!cb.record_failure());
        assert!(!cb.is_cooldown());
    }

    #[test]
    fn test_circuit_breaker_three_failures_trips() {
        let mut cb = WriteCircuitBreaker::new();
        assert!(!cb.record_failure());
        assert!(!cb.record_failure());
        assert!(cb.record_failure());
        assert!(cb.is_cooldown());
    }

    #[test]
    fn test_circuit_breaker_success_resets() {
        let mut cb = WriteCircuitBreaker::new();
        let _ = cb.record_failure();
        let _ = cb.record_failure();
        let _ = cb.record_failure();
        assert!(cb.is_cooldown());

        cb.record_success();
        assert!(!cb.is_cooldown());
        assert_eq!(cb.fail_count, 0);
    }

    #[test]
    fn test_watchdog_initial_healthy() {
        let mut wd = GpuWatchdog::new();
        assert_eq!(wd.check(100), GpuHealth::Healthy);
    }

    #[test]
    fn test_watchdog_stall_detected() {
        let mut wd = GpuWatchdog::new();
        // Override last_check to simulate immediate check
        wd.last_check = Instant::now() - GpuWatchdog::POLL_INTERVAL;

        for _ in 0..GpuWatchdog::STALL_THRESHOLD - 1 {
            assert_eq!(wd.check(500), GpuHealth::Healthy);
            wd.last_check = Instant::now() - GpuWatchdog::POLL_INTERVAL;
        }
        assert_eq!(wd.check(500), GpuHealth::Stalled);
    }

    #[test]
    fn test_watchdog_freq_change_clears_stall() {
        let mut wd = GpuWatchdog::new();
        wd.last_check = Instant::now() - GpuWatchdog::POLL_INTERVAL;
        assert_eq!(wd.check(100), GpuHealth::Healthy);
        wd.last_check = Instant::now() - GpuWatchdog::POLL_INTERVAL;
        assert_eq!(wd.check(100), GpuHealth::Healthy);
        // Frequency changes
        wd.last_check = Instant::now() - GpuWatchdog::POLL_INTERVAL;
        assert_eq!(wd.check(200), GpuHealth::Healthy);
        // Should be reset
        wd.last_check = Instant::now() - GpuWatchdog::POLL_INTERVAL;
        assert_eq!(wd.check(200), GpuHealth::Healthy);
    }
}
