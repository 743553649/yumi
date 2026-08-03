package com.yumi.bridge.utils;

/**
 * Utility class for parsing CPU scaling frequencies and /proc/stat CPU statistics.
 */
public final class CpuStatsParser {

    private CpuStatsParser() {
        // Private constructor for utility class
    }

    /**
     * Data snapshot holding CPU total time and idle time at a point in time.
     */
    public static class CpuStatSnapshot {
        private final int cpuId;
        private final long totalTime;
        private final long idleTime;

        public CpuStatSnapshot(int cpuId, long totalTime, long idleTime) {
            this.cpuId = cpuId;
            this.totalTime = totalTime;
            this.idleTime = idleTime;
        }

        public int getCpuId() {
            return cpuId;
        }

        public long getTotalTime() {
            return totalTime;
        }

        public long getIdleTime() {
            return idleTime;
        }
    }

    /**
     * Converts frequency in kHz to MHz.
     *
     * @param kHz Frequency value in kHz
     * @return Frequency value in MHz
     */
    public static long khzToMhz(long kHz) {
        if (kHz <= 0) return 0;
        return kHz / 1000L;
    }

    /**
     * Parses a raw line containing frequency in kHz (e.g. from sysfs scaling_cur_freq) to MHz.
     *
     * @param line Raw input string from sysfs
     * @return Frequency in MHz, or 0 if invalid
     */
    public static long parseFreqLineToMhz(String line) {
        if (line == null) return 0L;
        String trimmed = line.trim();
        if (trimmed.isEmpty()) return 0L;
        try {
            long kHz = Long.parseLong(trimmed);
            return khzToMhz(kHz);
        } catch (NumberFormatException e) {
            return 0L;
        }
    }

    /**
     * Parses a line from /proc/stat for a specific CPU core.
     *
     * @param line A line starting with 'cpu' followed by numbers
     * @return CpuStatSnapshot object if parsed successfully, or null
     */
    public static CpuStatSnapshot parseStatLine(String line) {
        if (line == null) return null;
        String trimmed = line.trim();
        if (!trimmed.startsWith("cpu")) return null;

        String[] parts = trimmed.split("\\s+");
        if (parts.length < 5) return null;

        try {
            String cpuLabel = parts[0].substring(3);
            if (cpuLabel.isEmpty()) return null; // "cpu" global line, ignore
            int cpuId = Integer.parseInt(cpuLabel);

            long user = Long.parseLong(parts[1]);
            long nice = Long.parseLong(parts[2]);
            long system = Long.parseLong(parts[3]);
            long idle = Long.parseLong(parts[4]);
            long iowait = parts.length > 5 ? Long.parseLong(parts[5]) : 0;
            long irq = parts.length > 6 ? Long.parseLong(parts[6]) : 0;
            long softirq = parts.length > 7 ? Long.parseLong(parts[7]) : 0;
            long steal = parts.length > 8 ? Long.parseLong(parts[8]) : 0;

            long total = user + nice + system + idle + iowait + irq + softirq + steal;
            long idleTotal = idle + iowait;

            return new CpuStatSnapshot(cpuId, total, idleTotal);
        } catch (NumberFormatException e) {
            return null;
        }
    }

    /**
     * Calculates usage percentage (0..100) between two stat snapshots.
     *
     * @param prev Previous snapshot
     * @param cur Current snapshot
     * @return Usage percentage from 0 to 100
     */
    public static int calculateUsage(CpuStatSnapshot prev, CpuStatSnapshot cur) {
        if (prev == null || cur == null) return 0;
        long diffTotal = cur.getTotalTime() - prev.getTotalTime();
        long diffIdle = cur.getIdleTime() - prev.getIdleTime();

        if (diffTotal <= 0) return 0;
        long busy = diffTotal - diffIdle;
        long usage = (busy * 100) / diffTotal;
        return (int) Math.max(0, Math.min(100, usage));
    }
}
