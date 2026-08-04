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
}
