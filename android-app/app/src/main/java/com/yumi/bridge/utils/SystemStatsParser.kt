package com.yumi.bridge.utils

import java.util.Locale

/**
 * Utility object for parsing /proc/meminfo memory statistics and battery telemetry.
 */
object SystemStatsParser {

    data class MemoryStats(
        val ramPercent: Int,
        val ramDetailText: String,
        val swapPercent: Int,
        val swapDetailText: String
    )

    /**
     * Parses content of /proc/meminfo to extract RAM & Swap metrics.
     *
     * @param memInfoContent Raw text from /proc/meminfo
     * @return MemoryStats object containing percentage and detailed format strings
     */
    @JvmStatic
    fun parseMemInfo(memInfoContent: String?): MemoryStats {
        if (memInfoContent == null || memInfoContent.isEmpty()) {
            return MemoryStats(0, "0.0G / 0.0G", 0, "0.0G / 0.0G")
        }

        var memTotalKb = 0L
        var memAvailableKb = 0L
        var swapTotalKb = 0L
        var swapFreeKb = 0L

        for (line in memInfoContent.split("\n")) {
            val l = line.trim()
            when {
                l.startsWith("MemTotal:") -> memTotalKb = parseKbValue(l)
                l.startsWith("MemAvailable:") -> memAvailableKb = parseKbValue(l)
                l.startsWith("SwapTotal:") -> swapTotalKb = parseKbValue(l)
                l.startsWith("SwapFree:") -> swapFreeKb = parseKbValue(l)
            }
        }

        // RAM Calculation
        val memUsedKb = maxOf(0L, memTotalKb - memAvailableKb)
        val ramPercent = if (memTotalKb > 0L) ((memUsedKb * 100L) / memTotalKb).toInt() else 0
        val ramUsedGb = memUsedKb / (1024.0 * 1024.0)
        val ramTotalGb = memTotalKb / (1024.0 * 1024.0)
        val ramDetailText = String.format(Locale.getDefault(), "%.1fG / %.1fG", ramUsedGb, ramTotalGb)

        // Swap Calculation
        val swapUsedKb = maxOf(0L, swapTotalKb - swapFreeKb)
        val swapPercent = if (swapTotalKb > 0L) ((swapUsedKb * 100L) / swapTotalKb).toInt() else 0
        val swapUsedGb = swapUsedKb / (1024.0 * 1024.0)
        val swapTotalGb = swapTotalKb / (1024.0 * 1024.0)
        val swapDetailText = String.format(Locale.getDefault(), "%.1fG / %.1fG", swapUsedGb, swapTotalGb)

        return MemoryStats(ramPercent, ramDetailText, swapPercent, swapDetailText)
    }

    private fun parseKbValue(line: String): Long {
        return try {
            val parts = line.split("\\s+".toRegex())
            if (parts.size >= 2) parts[1].toLong() else 0L
        } catch (e: NumberFormatException) {
            0L
        }
    }

    /**
     * Formats raw battery current (µA or mA) and voltage (µV or mV) to Watts (W).
     * Automatically normalizes scale if raw current is reported in mA (e.g. -350 mA vs -350000 µA).
     *
     * @param rawCurrent Current value in µA or mA
     * @param rawVoltage Voltage value in µV or mV
     * @return Formatted power text in W (e.g. "-2.5 W" or "+12.0 W")
     */
    @JvmStatic
    fun formatPowerWatts(rawCurrent: Long, rawVoltage: Long, isCharging: Boolean): String {
        if (rawCurrent == 0L) {
            return "0.0 W"
        }

        var currentUa = rawCurrent
        if (Math.abs(rawCurrent) > 0L && Math.abs(rawCurrent) < 10000L) {
            currentUa = rawCurrent * 1000L
        }

        var voltageUv = rawVoltage
        if (voltageUv <= 0L) {
            voltageUv = 4000000L
        } else if (voltageUv < 10000L) {
            voltageUv = voltageUv * 1000L
        }

        val currentAmps = currentUa / 1000000.0
        val voltageVolts = voltageUv / 1000000.0
        val watts = Math.abs(currentAmps * voltageVolts)

        if (watts < 0.05) {
            return "0.0 W"
        }

        return if (isCharging) {
            String.format(Locale.getDefault(), "+%.1f W", watts)
        } else {
            String.format(Locale.getDefault(), "-%.1f W", watts)
        }
    }

    @JvmStatic
    fun formatPowerWatts(rawCurrent: Long, rawVoltage: Long): String =
        formatPowerWatts(rawCurrent, rawVoltage, false)

    /**
     * Formats battery temperature from tenths of degrees Celsius.
     *
     * @param tempTenths Temperature in 0.1°C (e.g. 365 = 36.5°C)
     * @return Formatted string (e.g. "36.5 ℃")
     */
    @JvmStatic
    fun formatTemperature(tempTenths: Int): String {
        val tempC = tempTenths / 10.0
        return String.format(Locale.getDefault(), "%.1f ℃", tempC)
    }
}
