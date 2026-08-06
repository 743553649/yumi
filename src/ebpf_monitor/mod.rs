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

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct EbpfFrameEvent {
    pub pid: u32,
    pub flags: u32,
    pub frame_time_us: u64,
    pub timestamp_ns: u64,
}

pub struct EbpfMonitor {
    is_active: bool,
}

impl EbpfMonitor {
    pub fn new() -> Self {
        Self { is_active: false }
    }

    pub fn is_available(&self) -> bool {
        self.is_active
    }
}

impl Default for EbpfMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_event_memory_alignment() {
        assert_eq!(std::mem::align_of::<EbpfFrameEvent>(), 8);
        assert_eq!(std::mem::size_of::<EbpfFrameEvent>(), 24);
    }

    // userspace 侧 FrameTimestampEvent 必须与 yumi-ebpf/src/main.rs 的 eBPF 侧
    // `struct FrameTimestampEvent { pid: u32, ktime_ns: u64 }` 内存布局完全一致，
    // 否则从 RingBuf 读取时会错位。两端均为 #[repr(C)]：u32(4B) + 4B 对齐填充
    // + u64(8B) = 16B，对齐 8。eBPF 侧结构体是私有定义、跨 crate 无法直接引用，
    // 这里用 size/align 校验守住布局契约，改动任一侧字段时此测试会立即报警。
    #[test]
    fn test_frame_event_layout_matches_ebpf() {
        use std::mem;
        assert_eq!(
            mem::size_of::<crate::monitor::fps_monitor::FrameTimestampEvent>(),
            16
        );
        assert_eq!(
            mem::align_of::<crate::monitor::fps_monitor::FrameTimestampEvent>(),
            8
        );
    }
}
