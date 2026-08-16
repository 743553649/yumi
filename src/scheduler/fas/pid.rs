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

pub(super) struct PidController {
    // 用户配置的基准系数 (基于 60fps 场景调优)
    pub(super) base_kp: f32,
    pub(super) base_ki: f32,
    pub(super) base_kd: f32,
    // 运行时实际使用的动态系数 (根据 target_fps 和场景自动缩放)
    kp: f32,
    ki: f32,
    kd: f32,
    integral: f32,
    prev_error: f32,
    filtered_deriv: f32,
    integral_limit: f32,
    // 缓存当前适配的目标帧率，避免重复计算
    adapted_fps: f32,
}

impl PidController {
    pub(super) fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            base_kp: kp,
            base_ki: ki,
            base_kd: kd,
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
            filtered_deriv: 0.0,
            integral_limit: 0.15,
            adapted_fps: 60.0,
        }
    }

    /// 根据 target_fps 动态缩放 PID 系数
    ///
    /// 核心思想:
    /// 高刷下帧间隔 budget 更短 (144fps → 6.9ms vs 60fps → 16.7ms)，
    /// 同样 1ms 的帧时间偏差在高刷下"严重程度"更高，
    /// 因此 P/I/D 三个通道的增益都需要随 target_fps 缩放，
    /// 但缩放系数不同：P 最激进，D 最保守 (高刷噪声大)。
    pub(super) fn adapt_to_target_fps(&mut self, target_fps: f32) {
        // 防御非法 target_fps（0/负/NaN/Inf），避免 PID 系数与积分限幅被污染
        if !target_fps.is_finite() || target_fps <= 0.0 {
            return;
        }
        if (target_fps - self.adapted_fps).abs() < 0.5 {
            return;
        }
        self.adapted_fps = target_fps;

        let ratio = target_fps / 60.0;
        // kp: 线性缩放 — 高刷时每 ms 偏差代表更大的帧率损失
        self.kp = self.base_kp * ratio;
        // ki: sqrt 缩放 — 高刷帧多，积分器积累更快，弱化以防过冲
        self.ki = self.base_ki * ratio.sqrt();
        // kd: 保守 0.3 次幂 — 高刷帧间噪声更大，微分项放大噪声
        self.kd = self.base_kd * ratio.powf(0.3);

        // 积分限幅：高刷下缩小，防止积分器饱和导致频率虚高
        self.integral_limit = 0.15 * (60.0 / target_fps.max(1.0)).sqrt();
        // 不 reset 积分器（保持连续性），只做 clamp
        self.integral = self
            .integral
            .clamp(-self.integral_limit, self.integral_limit);
    }

    /// 带利用率感知的 PID 计算
    ///
    /// 当前台线程 CPU 利用率很低时，说明瓶颈不在 CPU（可能是 GPU bound
    /// 或 IO bound），此时 PID 拉频不会改善帧率，反而白给功耗。
    /// 通过 util_gain 衰减 P 项增益，避免无效拉频。
    pub(super) fn compute(&mut self, error: f32, inst_error: f32, norm: f32, fg_util: f32) -> f32 {
        let safe_norm = norm.clamp(0.5, 2.5);

        if error < 0.0 {
            self.integral += error * safe_norm;
        } else {
            let leak = (0.70 + safe_norm * 0.08).clamp(0.70, 0.85);
            self.integral *= leak;
        }
        let dyn_limit = self.integral_limit * safe_norm.clamp(0.7, 1.3);
        self.integral = self.integral.clamp(-dyn_limit, dyn_limit);

        let raw_deriv = (error - self.prev_error) / safe_norm;
        // 动态低通滤波：高刷下帧间微小抖动（调度噪声）在微秒级被放大，
        // 固定 0.7/0.3 滤波器在 144fps 下无法有效抑制。
        // alpha 随 target_fps 升高而降低：60fps=0.30, 120fps=0.21, 144fps=0.19
        // 使 D 项在高刷下更加平滑，避免输出高频震荡。
        let d_alpha = (0.30 * (60.0 / self.adapted_fps.max(1.0)).sqrt()).clamp(0.10, 0.30);
        self.filtered_deriv = self.filtered_deriv * (1.0 - d_alpha) + raw_deriv * d_alpha;
        self.prev_error = error;

        // 利用率感知增益调制
        // fg_util < 0.30 → GPU/IO bound，PID 增频无效，衰减 P 项
        // fg_util ∈ [0.30, 1.0] → CPU bound，正常增益
        // fg_util 无数据 (≤ 0.01) → 刚启动还没采样到，不衰减
        let util_gain = if fg_util > 0.01 && fg_util < 0.30 {
            0.3 + fg_util * 2.3 // 0.3 ~ 0.99
        } else {
            1.0
        };

        let p_term = self.kp * inst_error * util_gain;
        let i_term = self.ki * self.integral;
        let d_term = self.kd * self.filtered_deriv;

        p_term + i_term + d_term
    }

    pub(super) fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
        self.filtered_deriv = 0.0;
    }

    pub(super) fn update_coefficients(&mut self, kp: f32, ki: f32, kd: f32) {
        self.base_kp = kp;
        self.base_ki = ki;
        self.base_kd = kd;
        // 重新按当前 adapted_fps 缩放
        let fps = self.adapted_fps;
        self.adapted_fps = 0.0; // 强制刷新
        self.adapt_to_target_fps(fps);
        self.reset();
    }
}

// ════════════════════════════════════════════════════════════════
//  工具函数
// ════════════════════════════════════════════════════════════════

#[inline]
pub(super) fn fps_norm(target_fps: f32) -> f32 {
    (60.0 / target_fps.max(1.0)).sqrt()
}

#[inline]
pub(super) fn scale_frames(base: u32, target_fps: f32) -> u32 {
    ((base as f32 * target_fps / 60.0).max(base as f32 * 0.4)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-6;

    #[test]
    fn test_new_initial_values() {
        let pid = PidController::new(0.05, 0.01, 0.006);
        assert!((pid.base_kp - 0.05).abs() < EPSILON);
        assert!((pid.base_ki - 0.01).abs() < EPSILON);
        assert!((pid.base_kd - 0.006).abs() < EPSILON);
        assert!((pid.kp - 0.05).abs() < EPSILON);
        assert!((pid.ki - 0.01).abs() < EPSILON);
        assert!((pid.kd - 0.006).abs() < EPSILON);
        assert!((pid.integral).abs() < EPSILON);
        assert!((pid.prev_error).abs() < EPSILON);
        assert!((pid.filtered_deriv).abs() < EPSILON);
        assert!((pid.integral_limit - 0.15).abs() < EPSILON);
        assert!((pid.adapted_fps - 60.0).abs() < EPSILON);
    }

    #[test]
    fn test_adapt_invalid_input_defense() {
        let mut pid = PidController::new(0.05, 0.01, 0.006);
        let original_kp = pid.kp;
        let original_ki = pid.ki;
        let original_kd = pid.kd;

        // 0 应该被忽略
        pid.adapt_to_target_fps(0.0);
        assert!((pid.kp - original_kp).abs() < EPSILON);

        // 负数应该被忽略
        pid.adapt_to_target_fps(-1.0);
        assert!((pid.kp - original_kp).abs() < EPSILON);

        // NaN 应该被忽略
        pid.adapt_to_target_fps(f32::NAN);
        assert!((pid.kp - original_kp).abs() < EPSILON);

        // Inf 应该被忽略
        pid.adapt_to_target_fps(f32::INFINITY);
        assert!((pid.kp - original_kp).abs() < EPSILON);
    }

    #[test]
    fn test_adapt_change_threshold() {
        let mut pid = PidController::new(0.05, 0.01, 0.006);
        let original_fps = pid.adapted_fps;

        // 变化小于 0.5fps 不应该触发重新计算
        pid.adapt_to_target_fps(60.3);
        assert!((pid.adapted_fps - original_fps).abs() < EPSILON);

        // 变化大于 0.5fps 应该触发重新计算
        pid.adapt_to_target_fps(61.0);
        assert!((pid.adapted_fps - 61.0).abs() < EPSILON);
    }

    #[test]
    fn test_adapt_60_to_120_scaling() {
        let mut pid = PidController::new(0.05, 0.01, 0.006);
        pid.adapt_to_target_fps(120.0);

        // kp: 线性缩放 (120/60 = 2.0)
        let expected_kp = 0.05 * 2.0;
        assert!((pid.kp - expected_kp).abs() < 0.001);

        // ki: sqrt 缩放 (sqrt(2) ≈ 1.414)
        let expected_ki = 0.01 * (2.0_f32).sqrt();
        assert!((pid.ki - expected_ki).abs() < 0.001);

        // kd: 0.3 次幂缩放 (2^0.3 ≈ 1.231)
        let expected_kd = 0.006 * (2.0_f32).powf(0.3);
        assert!((pid.kd - expected_kd).abs() < 0.001);
    }

    #[test]
    fn test_compute_negative_error_integral() {
        let mut pid = PidController::new(0.05, 0.01, 0.006);
        let initial_integral = pid.integral;

        // 负误差应该积分积累
        pid.compute(-0.1, -0.1, 1.0, 0.5);
        assert!(pid.integral > initial_integral);
    }

    #[test]
    fn test_compute_positive_error_integral_decay() {
        let mut pid = PidController::new(0.05, 0.01, 0.006);

        // 先积累一些负误差
        pid.compute(-0.5, -0.5, 1.0, 0.5);
        let integral_before = pid.integral;

        // 正误差应该衰减积分
        pid.compute(0.1, 0.1, 1.0, 0.5);
        assert!(pid.integral < integral_before);
    }

    #[test]
    fn test_compute_fg_util_gain() {
        let mut pid = PidController::new(0.05, 0.01, 0.006);
        let mut pid2 = PidController::new(0.05, 0.01, 0.006);

        // 低 fg_util 应该产生较小的 P 项增益
        let output_low_util = pid.compute(-0.1, -0.1, 1.0, 0.15);
        let output_high_util = pid2.compute(-0.1, -0.1, 1.0, 0.5);

        // 低利用率时输出应该更小（P项被衰减）
        assert!(output_low_util.abs() < output_high_util.abs());
    }

    #[test]
    fn test_reset_clears_state() {
        let mut pid = PidController::new(0.05, 0.01, 0.006);

        // 先积累一些状态
        pid.compute(-0.5, -0.5, 1.0, 0.5);
        pid.compute(0.1, 0.1, 1.0, 0.5);

        // reset 应该清零
        pid.reset();
        assert!((pid.integral).abs() < EPSILON);
        assert!((pid.prev_error).abs() < EPSILON);
        assert!((pid.filtered_deriv).abs() < EPSILON);
    }

    #[test]
    fn test_update_coefficients_triggers_rescale() {
        let mut pid = PidController::new(0.05, 0.01, 0.006);
        pid.adapt_to_target_fps(120.0);

        // 更新系数
        pid.update_coefficients(0.1, 0.02, 0.012);

        // 应该按当前 fps (120) 重新缩放
        let expected_kp = 0.1 * 2.0; // 120/60 = 2.0
        assert!((pid.kp - expected_kp).abs() < 0.001);
        assert!((pid.base_kp - 0.1).abs() < EPSILON);

        // 状态应该被重置
        assert!((pid.integral).abs() < EPSILON);
    }

    #[test]
    fn test_fps_norm_boundary() {
        // target_fps = 1 应该返回 sqrt(60)
        let result = fps_norm(1.0);
        let expected = (60.0_f32).sqrt();
        assert!((result - expected).abs() < 0.001);

        // target_fps = 0 应该返回 sqrt(60) (max(0, 1.0) = 1.0)
        let result = fps_norm(0.0);
        assert!((result - expected).abs() < 0.001);

        // target_fps = 60 应该返回 1.0
        let result = fps_norm(60.0);
        assert!((result - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_scale_frames_minimum() {
        // base=100, fps=30 → 100*30/60=50, 100*0.4=40, max(50,40)=50
        let result = scale_frames(100, 30.0);
        assert_eq!(result, 50);

        // base=100, fps=15 → 100*15/60=25, 100*0.4=40, max(25,40)=40
        let result = scale_frames(100, 15.0);
        assert_eq!(result, 40);

        // base=100, fps=60 → 100*60/60=100
        let result = scale_frames(100, 60.0);
        assert_eq!(result, 100);
    }

    #[test]
    fn test_integral_limit_clamping() {
        let mut pid = PidController::new(0.05, 0.01, 0.006);

        // 连续负误差应该被限幅
        for _ in 0..100 {
            pid.compute(-1.0, -1.0, 1.0, 0.5);
        }

        // 积分应该被限制在 integral_limit 范围内
        assert!(pid.integral <= pid.integral_limit + EPSILON);
        assert!(pid.integral >= -pid.integral_limit - EPSILON);
    }
}
