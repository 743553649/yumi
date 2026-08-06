package com.yumi.bridge.utils

/**
 * Utility object for parsing CPU scaling frequencies and /proc/stat CPU statistics.
 */
object CpuStatsParser {

    /**
     * Data snapshot holding CPU total time and idle time at a point in time.
     */
    data class CpuStatSnapshot(
        val cpuId: Int,
        val totalTime: Long,
        val idleTime: Long
    )

    /**
     * Converts frequency in kHz to MHz.
     *
     * @param kHz Frequency value in kHz
     * @return Frequency value in MHz
     */
    @JvmStatic
    fun khzToMhz(kHz: Long): Long {
        if (kHz <= 0L) return 0L
        return kHz / 1000L
    }

    /**
     * Parses a raw line containing frequency in kHz (e.g. from sysfs scaling_cur_freq) to MHz.
     *
     * @param line Raw input string from sysfs
     * @return Frequency in MHz, or 0 if invalid
     */
    @JvmStatic
    fun parseFreqLineToMhz(line: String?): Long {
        if (line == null) return 0L
        val trimmed = line.trim()
        if (trimmed.isEmpty()) return 0L
        return try {
            val kHz = trimmed.toLong()
            khzToMhz(kHz)
        } catch (e: NumberFormatException) {
            0L
        }
    }

    /**
     * Parses a line from /proc/stat for a specific CPU core.
     *
     * @param line A line starting with 'cpu' followed by numbers
     * @return CpuStatSnapshot object if parsed successfully, or null
     */
    @JvmStatic
    fun parseStatLine(line: String?): CpuStatSnapshot? {
        if (line == null) return null
        val trimmed = line.trim()
        if (!trimmed.startsWith("cpu")) return null

        val parts = trimmed.split("\\s+".toRegex())
        if (parts.size < 5) return null

        return try {
            val cpuLabel = parts[0].substring(3)
            if (cpuLabel.isEmpty()) return null  // "cpu" global line, ignore
            val cpuId = cpuLabel.toInt()

            val user = parts[1].toLong()
            val nice = parts[2].toLong()
            val system = parts[3].toLong()
            val idle = parts[4].toLong()
            val iowait = if (parts.size > 5) parts[5].toLong() else 0L
            val irq = if (parts.size > 6) parts[6].toLong() else 0L
            val softirq = if (parts.size > 7) parts[7].toLong() else 0L
            val steal = if (parts.size > 8) parts[8].toLong() else 0L

            val total = user + nice + system + idle + iowait + irq + softirq + steal
            val idleTotal = idle + iowait

            CpuStatSnapshot(cpuId, total, idleTotal)
        } catch (e: NumberFormatException) {
            null
        }
    }

    /**
     * Calculates usage percentage (0..100) between two stat snapshots.
     *
     * @param prev Previous snapshot
     * @param cur Current snapshot
     * @return Usage percentage from 0 to 100
     */
    @JvmStatic
    fun calculateUsage(prev: CpuStatSnapshot?, cur: CpuStatSnapshot?): Int {
        if (prev == null || cur == null) return 0
        val diffTotal = cur.totalTime - prev.totalTime
        val diffIdle = cur.idleTime - prev.idleTime

        if (diffTotal <= 0L) return 0
        val busy = diffTotal - diffIdle
        val usage = (busy * 100L) / diffTotal
        return usage.coerceIn(0L, 100L).toInt()
    }
}
