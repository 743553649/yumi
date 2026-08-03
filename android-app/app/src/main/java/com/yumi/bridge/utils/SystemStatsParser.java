package com.yumi.bridge.utils;

import java.util.Locale;

/**
 * Utility class for parsing /proc/meminfo memory statistics and battery telemetry.
 */
public final class SystemStatsParser {

    private SystemStatsParser() {
        // Private constructor for utility class
    }

    public static class MemoryStats {
        private final int ramPercent;
        private final String ramDetailText;
        private final int swapPercent;
        private final String swapDetailText;

        public MemoryStats(int ramPercent, String ramDetailText, int swapPercent, String swapDetailText) {
            this.ramPercent = ramPercent;
            this.ramDetailText = ramDetailText;
            this.swapPercent = swapPercent;
            this.swapDetailText = swapDetailText;
        }

        public int getRamPercent() {
            return ramPercent;
        }

        public String getRamDetailText() {
            return ramDetailText;
        }

        public int getSwapPercent() {
            return swapPercent;
        }

        public String getSwapDetailText() {
            return swapDetailText;
        }
    }

    /**
     * Parses content of /proc/meminfo to extract RAM & Swap metrics.
     *
     * @param memInfoContent Raw text from /proc/meminfo
     * @return MemoryStats object containing percentage and detailed format strings
     */
    public static MemoryStats parseMemInfo(String memInfoContent) {
        if (memInfoContent == null || memInfoContent.isEmpty()) {
            return new MemoryStats(0, "0.0G / 0.0G", 0, "0.0G / 0.0G");
        }

        long memTotalKb = 0;
        long memAvailableKb = 0;
        long swapTotalKb = 0;
        long swapFreeKb = 0;

        String[] lines = memInfoContent.split("\n");
        for (String line : lines) {
            line = line.trim();
            if (line.startsWith("MemTotal:")) {
                memTotalKb = parseKbValue(line);
            } else if (line.startsWith("MemAvailable:")) {
                memAvailableKb = parseKbValue(line);
            } else if (line.startsWith("SwapTotal:")) {
                swapTotalKb = parseKbValue(line);
            } else if (line.startsWith("SwapFree:")) {
                swapFreeKb = parseKbValue(line);
            }
        }

        // RAM Calculation
        long memUsedKb = Math.max(0, memTotalKb - memAvailableKb);
        int ramPercent = memTotalKb > 0 ? (int) ((memUsedKb * 100) / memTotalKb) : 0;
        double ramUsedGb = memUsedKb / (1024.0 * 1024.0);
        double ramTotalGb = memTotalKb / (1024.0 * 1024.0);
        String ramDetailText = String.format(Locale.getDefault(), "%.1fG / %.1fG", ramUsedGb, ramTotalGb);

        // Swap Calculation
        long swapUsedKb = Math.max(0, swapTotalKb - swapFreeKb);
        int swapPercent = swapTotalKb > 0 ? (int) ((swapUsedKb * 100) / swapTotalKb) : 0;
        double swapUsedGb = swapUsedKb / (1024.0 * 1024.0);
        double swapTotalGb = swapTotalKb / (1024.0 * 1024.0);
        String swapDetailText = String.format(Locale.getDefault(), "%.1fG / %.1fG", swapUsedGb, swapTotalGb);

        return new MemoryStats(ramPercent, ramDetailText, swapPercent, swapDetailText);
    }

    private static long parseKbValue(String line) {
        try {
            String[] parts = line.split("\\s+");
            if (parts.length >= 2) {
                return Long.parseLong(parts[1]);
            }
        } catch (NumberFormatException ignored) {}
        return 0L;
    }

    /**
     * Formats microAmps and microVolts to Watts (W).
     *
     * @param currentUa Current in microamps (µA)
     * @param voltageUv Voltage in microvolts (µV)
     * @return Formatted power text in W (e.g. "-2.5 W" or "+12.0 W")
     */
    public static String formatPowerWatts(long currentUa, long voltageUv) {
        if (currentUa == 0 || voltageUv == 0) {
            return "0.0 W";
        }
        double currentAmps = currentUa / 1000000.0;
        double voltageVolts = voltageUv / 1000000.0;
        double watts = currentAmps * voltageVolts;
        return String.format(Locale.getDefault(), "%+.1f W", watts);
    }

    /**
     * Formats battery temperature from tenths of degrees Celsius.
     *
     * @param tempTenths Temperature in 0.1°C (e.g. 365 = 36.5°C)
     * @return Formatted string (e.g. "36.5 ℃")
     */
    public static String formatTemperature(int tempTenths) {
        double tempC = tempTenths / 10.0;
        return String.format(Locale.getDefault(), "%.1f ℃", tempC);
    }
}
