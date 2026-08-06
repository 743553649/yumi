package com.yumi.bridge.model

/**
 * 守护进程实时日志条目数据模型。
 * 从 MainActivity 上帝类中拆出（FIX-022）。
 */
data class RealLogEntry(
    val rawLine: String,
    val formattedChineseLine: String,
    val level: Int
)
