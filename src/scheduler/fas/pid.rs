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
    pub(super) base_kp: f32, pub(super) base_ki: f32, pub(super) base_kd: f32,
    // 运行时实际使用的动态系数 (根据 target_fps 和场景自动缩放)
    kp: f32, ki: f32, kd: f32,
    integral: f32, prev_error: f32,
    filtered_deriv: f32,
    integral_limit: f32,
    // 缓存当前适配的目标帧率，避免重复计算
    adapted_fps: f32,
}

impl PidController {
    pub(super) fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            base_kp: kp, base_ki: ki, base_kd: kd,
            kp, ki, kd,
            integral: 0.0, prev_error: 0.0,
            filtered_deriv: 0.0, integral_limit: 0.15,
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
        let target_fps = if target_fps.is_finite() && target_fps > 0.0 { target_fps } else { 60.0 };
        if (target_fps - self.adapted_fps).abs() < 0.5 { return; }
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
        self.integral = self.integral.clamp(-self.integral_limit, self.integral_limit);
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
        // fg_util < 0.45 → GPU/IO bound，PID 增频无效，衰减 P 项
        // fg_util ∈ [0.45, 1.0] → CPU bound，正常增益
        // fg_util 无数据 (≤ 0.01) → 刚启动还没采样到，不衰减
        let util_gain = if fg_util > 0.01 && fg_util < 0.45 {
            0.3 + fg_util * 1.56  // 0.3 ~ 1.0
        } else {
            1.0
        };

        let p_term = self.kp * inst_error * util_gain;
        let i_term = self.ki * self.integral;
        let d_term = self.kd * self.filtered_deriv;

        p_term + i_term + d_term
    }

    pub(super) fn reset(&mut self) {
        self.integral = 0.0; self.prev_error = 0.0; self.filtered_deriv = 0.0;
    }

    pub(super) fn update_coefficients(&mut self, kp: f32, ki: f32, kd: f32) {
        self.base_kp = kp; self.base_ki = ki; self.base_kd = kd;
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

// ════════════════════════════════════════════════════════════════
//  单元测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pid() -> PidController {
        PidController::new(1.0, 0.5, 0.2)
    }

    #[test]
    fn test_adapt_to_target_fps_scales_coefficients_up_for_high_refresh() {
        // 高刷下帧间隔更短，同样 1ms 偏差影响更大，三个通道增益都应放大
        // P 线性、I 开方、D 0.3 次幂——幅度递减但都 > 基准
        let mut pid = make_pid();
        pid.adapt_to_target_fps(144.0); // ratio = 2.4
        assert!(pid.kp > pid.base_kp, "kp 应随高刷放大: kp={} base={}", pid.kp, pid.base_kp);
        assert!(pid.ki > pid.base_ki, "ki 应随高刷放大");
        assert!(pid.kd > pid.base_kd, "kd 应随高刷放大");
        // P 增益最激进，D 最保守
        assert!(pid.kp / pid.base_kp > pid.kd / pid.base_kd);
    }

    #[test]
    fn test_adapt_to_target_fps_idempotent() {
        // 对同一 target_fps 重复调用（差值 < 0.5）应直接 return，系数不变
        let mut pid = make_pid();
        pid.adapt_to_target_fps(120.0);
        let (kp, ki, kd, limit) = (pid.kp, pid.ki, pid.kd, pid.integral_limit);
        pid.adapt_to_target_fps(120.0);
        assert_eq!(pid.kp, kp);
        assert_eq!(pid.ki, ki);
        assert_eq!(pid.kd, kd);
        assert_eq!(pid.integral_limit, limit);
    }

    #[test]
    fn test_adapt_invalid_fps_falls_back_to_60() {
        let mut pid = make_pid();
        pid.adapt_to_target_fps(144.0); // 先放大 kp
        assert!(pid.kp > pid.base_kp);
        // 非法 fps（0、NaN、负数）应回退到 60，系数恢复基准
        pid.adapt_to_target_fps(0.0);
        assert!((pid.adapted_fps - 60.0).abs() < 1e-6);
        assert!((pid.kp - pid.base_kp).abs() < 1e-6);
        pid.adapt_to_target_fps(f32::NAN);
        assert!((pid.kp - pid.base_kp).abs() < 1e-6);
        pid.adapt_to_target_fps(-10.0);
        assert!((pid.kp - pid.base_kp).abs() < 1e-6);
    }

    #[test]
    fn test_negative_error_integral_clamped_within_limit() {
        // 连续负 error（实际帧率高于目标）会累积 integral，
        // 但 dyn_limit 限幅必须生效，避免积分饱和导致频率虚高
        let mut pid = PidController::new(0.0, 1.0, 0.0); // 只看 I 项
        pid.adapt_to_target_fps(60.0); // integral_limit = 0.15
        for _ in 0..500 {
            pid.compute(-1.0, 0.0, 1.0, 1.0); // fg_util=1.0 不触发 P 衰减
        }
        let limit = pid.integral_limit;
        assert!(pid.integral >= -limit - 1e-6,
            "integral {} 不应低于 -limit {}", pid.integral, -limit);
        assert!(pid.integral <= limit + 1e-6,
            "integral {} 不应超过 +limit {}", pid.integral, limit);
    }

    #[test]
    fn test_low_fg_util_attenuates_p_term() {
        // 前台 CPU 利用率低（< 0.45）说明瓶颈不在 CPU，
        // util_gain 应衰减 P 项，避免无效拉频。
        // 用 ki=kd=0 隔离 P 项，两次单次 compute 对比
        let mut pid_low = PidController::new(1.0, 0.0, 0.0);
        let out_low = pid_low.compute(0.1, 0.1, 1.0, 0.2); // util_gain = 0.3+0.2*1.56 = 0.612

        let mut pid_high = PidController::new(1.0, 0.0, 0.0);
        let out_high = pid_high.compute(0.1, 0.1, 1.0, 0.8); // util_gain = 1.0

        assert!(out_low < out_high,
            "低利用率下 P 项应被衰减: low={} high={}", out_low, out_high);
        // 衰减比例应严格符合公式
        assert!((out_low / out_high - 0.612).abs() < 1e-3,
            "衰减比例应为 0.612，实际 {}", out_low / out_high);
    }

    #[test]
    fn test_no_util_data_does_not_attenuate() {
        // fg_util ≤ 0.01 视为刚启动未采样到，不应衰减（util_gain = 1.0）
        let mut pid_zero = PidController::new(1.0, 0.0, 0.0);
        let out_zero = pid_zero.compute(0.1, 0.1, 1.0, 0.0);

        let mut pid_normal = PidController::new(1.0, 0.0, 0.0);
        let out_normal = pid_normal.compute(0.1, 0.1, 1.0, 0.8);

        assert!((out_zero - out_normal).abs() < 1e-6, "无利用率数据不应衰减 P 项");
    }

    #[test]
    fn test_reset_clears_runtime_state() {
        let mut pid = make_pid();
        // 负 error 会累积 integral；非零 inst_error 会建立 prev_error / filtered_deriv
        pid.compute(-0.5, -0.3, 1.0, 0.8);
        assert!(pid.integral.abs() > 0.0, "integral 应已累积");
        assert!(pid.prev_error.abs() > 0.0);
        assert!(pid.filtered_deriv.abs() > 0.0);

        pid.reset();
        assert!((pid.integral - 0.0).abs() < 1e-6);
        assert!((pid.prev_error - 0.0).abs() < 1e-6);
        assert!((pid.filtered_deriv - 0.0).abs() < 1e-6);
        // base 系数与 adapted_fps 不应被 reset 影响
        assert!((pid.base_kp - 1.0).abs() < 1e-6);
        assert!((pid.adapted_fps - 60.0).abs() < 1e-6);
    }

    #[test]
    fn test_update_coefficients_rescales_and_resets() {
        let mut pid = make_pid();
        pid.adapt_to_target_fps(120.0); // ratio = 2.0
        pid.compute(-0.5, 0.0, 1.0, 0.8);
        assert!(pid.integral.abs() > 0.0);

        pid.update_coefficients(2.0, 1.0, 0.5);
        // base 系数更新
        assert!((pid.base_kp - 2.0).abs() < 1e-6);
        assert!((pid.base_ki - 1.0).abs() < 1e-6);
        assert!((pid.base_kd - 0.5).abs() < 1e-6);
        // 按 120fps 重新缩放: kp = base_kp * (120/60) = 4.0
        assert!((pid.kp - 4.0).abs() < 1e-6);
        // update_coefficients 内部调用了 reset
        assert!((pid.integral - 0.0).abs() < 1e-6);
        assert!((pid.prev_error - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_fps_norm_scales_with_refresh() {
        // 60fps → 1.0（基准）
        assert!((fps_norm(60.0) - 1.0).abs() < 1e-6);
        // 120fps → sqrt(0.5) ≈ 0.707（高刷下 norm 更小）
        assert!((fps_norm(120.0) - (0.5_f32).sqrt()).abs() < 1e-6);
        // 极小/非法 fps 由 max(1.0) 兜底，不产生 NaN
        assert!(fps_norm(0.0).is_finite());
        assert!(fps_norm(-1.0).is_finite());
    }

    #[test]
    fn test_scale_frames_linear_with_floor() {
        // 60fps → base 原值
        assert_eq!(scale_frames(10, 60.0), 10);
        // 120fps → base * 2
        assert_eq!(scale_frames(10, 120.0), 20);
        // 30fps → base * 0.5（未触底下限）
        assert_eq!(scale_frames(10, 30.0), 5);
        // 极低帧率不低于 base * 0.4 下限
        assert_eq!(scale_frames(10, 1.0), 4);
    }
}
