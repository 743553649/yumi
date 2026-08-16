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

const WINDOW_SIZE: usize = 120;

pub(super) struct FpsWindow {
    buf: [f32; WINDOW_SIZE],
    pos: usize,
    len: usize,
    sum: f32,
    sq_sum: f32,
    push_count: u32,
}

impl FpsWindow {
    pub(super) fn new() -> Self {
        Self {
            buf: [0.0; WINDOW_SIZE],
            pos: 0,
            len: 0,
            sum: 0.0,
            sq_sum: 0.0,
            push_count: 0,
        }
    }

    pub(super) fn push(&mut self, fps: f32) {
        if self.len == WINDOW_SIZE {
            let old = self.buf[self.pos];
            self.sum -= old;
            self.sq_sum -= old * old;
        } else {
            self.len += 1;
        }
        self.buf[self.pos] = fps;
        self.sum += fps;
        self.sq_sum += fps * fps;
        self.pos = (self.pos + 1) % WINDOW_SIZE;
        self.push_count += 1;
        // 从 512 降低到 64 帧校准一次，WINDOW_SIZE=120 下每半圈重算一次
        // 在 144fps 下约 0.44 秒校准一次，有效抑制浮点累积误差对齿轮决策的影响
        if self.push_count >= 64 {
            self.recalculate();
            self.push_count = 0;
        }
    }

    fn recalculate(&mut self) {
        let slice = &self.buf[..self.len];
        self.sum = slice.iter().sum();
        self.sq_sum = slice.iter().map(|x| x * x).sum();
    }

    #[inline]
    pub(super) fn count(&self) -> usize {
        self.len
    }
    #[inline]
    pub(super) fn mean(&self) -> f32 {
        if self.len == 0 {
            0.0
        } else {
            self.sum / self.len as f32
        }
    }

    pub(super) fn recent_mean(&self, n: usize) -> f32 {
        if self.len == 0 {
            return 0.0;
        }
        let count = n.min(self.len);
        let mut sum = 0.0;
        for i in 0..count {
            let idx = (self.pos + WINDOW_SIZE - 1 - i) % WINDOW_SIZE;
            sum += self.buf[idx];
        }
        sum / count as f32
    }

    pub(super) fn stddev(&self) -> f32 {
        if self.len < 2 {
            return 0.0;
        }
        let n = self.len as f32;
        let mean = self.sum / n;
        (self.sq_sum / n - mean * mean).max(0.0).sqrt()
    }

    pub(super) fn clear(&mut self) {
        self.buf = [0.0; WINDOW_SIZE];
        self.pos = 0;
        self.len = 0;
        self.sum = 0.0;
        self.sq_sum = 0.0;
        self.push_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn test_new_initial_state() {
        let window = FpsWindow::new();
        assert_eq!(window.count(), 0);
        assert!((window.mean()).abs() < EPSILON);
        assert!((window.stddev()).abs() < EPSILON);
    }

    #[test]
    fn test_push_partial_fill() {
        let mut window = FpsWindow::new();

        for i in 0..10 {
            window.push(60.0 + i as f32);
        }

        assert_eq!(window.count(), 10);
        // 均值应该是 (60+61+...+69)/10 = 64.5
        assert!((window.mean() - 64.5).abs() < 0.01);
    }

    #[test]
    fn test_push_overflow_overwrite() {
        let mut window = FpsWindow::new();

        // 推入 130 个值，窗口只能容纳 120
        for i in 0..130 {
            window.push(i as f32);
        }

        assert_eq!(window.count(), 120);
        // 最旧的 10 个值 (0-9) 被丢弃，窗口包含 10-129
        let expected_mean: f32 = (10..130).map(|x| x as f32).sum::<f32>() / 120.0;
        assert!((window.mean() - expected_mean).abs() < 0.1);
    }

    #[test]
    fn test_mean_empty_window() {
        let window = FpsWindow::new();
        assert!((window.mean()).abs() < EPSILON);
    }

    #[test]
    fn test_recent_mean_exceeds_length() {
        let mut window = FpsWindow::new();

        // 只推入 5 个值
        for i in 0..5 {
            window.push(60.0 + i as f32);
        }

        // 请求最近 200 个，但只有 5 个
        let recent = window.recent_mean(200);
        // 应该只取 5 个：64, 63, 62, 61, 60 (倒序)
        let expected = (60.0 + 61.0 + 62.0 + 63.0 + 64.0) / 5.0;
        assert!((recent - expected).abs() < 0.01);
    }

    #[test]
    fn test_recent_mean_order() {
        let mut window = FpsWindow::new();

        // 推入不同的值
        window.push(10.0);
        window.push(20.0);
        window.push(30.0);

        // 最近 1 个应该是 30.0
        let recent1 = window.recent_mean(1);
        assert!((recent1 - 30.0).abs() < EPSILON);

        // 最近 2 个应该是 (30 + 20) / 2 = 25.0
        let recent2 = window.recent_mean(2);
        assert!((recent2 - 25.0).abs() < EPSILON);
    }

    #[test]
    fn test_stddev_all_same() {
        let mut window = FpsWindow::new();

        // 推入 120 个相同的值
        for _ in 0..120 {
            window.push(60.0);
        }

        assert!((window.stddev()).abs() < EPSILON);
    }

    #[test]
    fn test_stddev_less_than_two() {
        let mut window = FpsWindow::new();

        // 只有 1 个值
        window.push(60.0);
        assert!((window.stddev()).abs() < EPSILON);

        // 0 个值
        let window2 = FpsWindow::new();
        assert!((window2.stddev()).abs() < EPSILON);
    }

    #[test]
    fn test_clear_resets_all() {
        let mut window = FpsWindow::new();

        // 推入一些值
        for i in 0..50 {
            window.push(60.0 + i as f32);
        }

        // 清空
        window.clear();

        assert_eq!(window.count(), 0);
        assert!((window.mean()).abs() < EPSILON);
        assert_eq!(window.pos, 0);
    }

    #[test]
    fn test_float_precision_compensation() {
        let mut window = FpsWindow::new();

        // 推入 10000 个相同的值（会触发多次 recalculate）
        for _ in 0..10000 {
            window.push(60.0);
        }

        // 均值应该仍然是 60.0，没有明显漂移
        assert!((window.mean() - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_circular_buffer_wrap() {
        let mut window = FpsWindow::new();

        // 推入 119 个值，pos = 119
        for i in 0..119 {
            window.push(i as f32);
        }
        assert_eq!(window.pos, 119);

        // 再推入一个，pos 应该回绕到 0
        window.push(119.0);
        assert_eq!(window.pos, 0);

        // 继续推入
        window.push(120.0);
        assert_eq!(window.pos, 1);
    }
}
