package com.yumi.bridge.model;

/**
 * 守护进程实时日志条目数据模型。
 * 从 MainActivity 上帝类中拆出（FIX-022）。
 */
public final class RealLogEntry {
    public final String rawLine;
    public final String formattedChineseLine;
    public final int level;

    public RealLogEntry(String rawLine, String formattedChineseLine, int level) {
        this.rawLine = rawLine;
        this.formattedChineseLine = formattedChineseLine;
        this.level = level;
    }
}
