package com.yumi.bridge.system;

import com.yumi.bridge.ui.compose.ComposeHomeBridgeKt;
import com.yumi.bridge.utils.CpuStatsParser;
import com.yumi.bridge.utils.SystemStatsParser;

import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.os.BatteryManager;
import android.os.Handler;

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.InputStreamReader;
import java.util.Locale;

/**
 * 系统仪表盘数据采集器。从 MainActivity 上帝类中拆出（FIX-022）。
 *
 * 职责：周期性读取 RAM/Swap、电池、运行时长、8 核 CPU 占用与频率，
 * 并通过 mainHandler 把汇总数据回投到主线程渲染到 Compose。
 *
 * CPU 占用率依赖前一次采样（prevCpuTotal/prevCpuIdle），状态由本类持有。
 * updateDashboard 应在后台线程调用（含阻塞 IO）。
 */
public class SystemDashboardMonitor {

    private final Context context;
    private final Handler mainHandler;
    private final long[] prevCpuTotal = new long[8];
    private final long[] prevCpuIdle = new long[8];

    public SystemDashboardMonitor(Context context, Handler mainHandler) {
        this.context = context;
        this.mainHandler = mainHandler;
    }

    /**
     * 在后台线程执行：读取全部系统指标，主线程回投渲染。
     *
     * @param currentMode 当前全局调度模式（用于首页状态展示）
     */
    public void updateDashboard(final String currentMode) {
        int ramPercent = 0;
        String ramDetailText = "0.0G / 0.0G";
        int swapPercent = 0;
        String swapDetailText = "0.0G / 0.0G";

        int batteryLevel = 100;
        String batteryTempText = "0.0 ℃";
        String batteryPowerText = "0.0 W";

        String uptimeText = "0天:00小时:00分钟";
        int[] usagePercents = new int[8];
        long[] curFreqs = new long[8];

        // 1. Read RAM & Swap memory stats (/proc/meminfo)
        try {
            String memInfo = readProcMemInfo();
            SystemStatsParser.MemoryStats memStats = SystemStatsParser.parseMemInfo(memInfo);
            ramPercent = memStats.getRamPercent();
            ramDetailText = memStats.getRamDetailText();
            swapPercent = memStats.getSwapPercent();
            swapDetailText = memStats.getSwapDetailText();
        } catch (Exception ignored) {}

        // 2. Read real-time battery stats (Level, Power, Temperature)
        try {
            long currentRaw = 0;
            long voltageUv = 4000000L;
            boolean isCharging = false;

            IntentFilter ifilter = new IntentFilter(Intent.ACTION_BATTERY_CHANGED);
            Intent batteryStatus = context.registerReceiver(null, ifilter);
            if (batteryStatus != null) {
                int status = batteryStatus.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
                isCharging = status == BatteryManager.BATTERY_STATUS_CHARGING ||
                             status == BatteryManager.BATTERY_STATUS_FULL;

                int level = batteryStatus.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
                int scale = batteryStatus.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
                if (level >= 0 && scale > 0) {
                    batteryLevel = (level * 100) / scale;
                }

                int temp = batteryStatus.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, 0);
                batteryTempText = SystemStatsParser.formatTemperature(temp);

                int voltageMv = batteryStatus.getIntExtra(BatteryManager.EXTRA_VOLTAGE, 4000);
                if (voltageMv > 0) {
                    voltageUv = voltageMv * 1000L;
                }
            }

            BatteryManager bm = (BatteryManager) context.getSystemService(Context.BATTERY_SERVICE);
            currentRaw = readBatteryCurrentNow(bm);
            batteryPowerText = SystemStatsParser.formatPowerWatts(currentRaw, voltageUv, isCharging);
        } catch (Exception ignored) {}

        // 3. System uptime (format: Days:Hours:Minutes)
        try {
            long elapsedSec = android.os.SystemClock.elapsedRealtime() / 1000;
            long days = elapsedSec / 86400;
            long hours = (elapsedSec % 86400) / 3600;
            long mins = (elapsedSec % 3600) / 60;
            uptimeText = String.format(Locale.getDefault(), "%d天:%02d小时:%02d分钟", days, hours, mins);
        } catch (Exception ignored) {}

        // 4. Read 8 CPU core dynamic stats
        try {
            readCpuStatsAndFreqs(usagePercents, curFreqs);
        } catch (Exception ignored) {}

        final int finalRamPercent = ramPercent;
        final String finalRamDetailText = ramDetailText;
        final int finalSwapPercent = swapPercent;
        final String finalSwapDetailText = swapDetailText;
        final int finalBatteryLevel = batteryLevel;
        final String finalBatteryTempText = batteryTempText;
        final String finalBatteryPowerText = batteryPowerText;
        final String finalUptimeText = uptimeText;

        mainHandler.post(new Runnable() {
            @Override
            public void run() {
                ComposeHomeBridgeKt.updateHomeScreenState(
                        currentMode,
                        curFreqs,
                        usagePercents,
                        finalRamPercent,
                        finalRamDetailText,
                        finalSwapPercent,
                        finalSwapDetailText,
                        finalBatteryLevel,
                        finalBatteryTempText,
                        finalBatteryPowerText,
                        finalUptimeText,
                        true
                );
            }
        });
    }

    private String readProcMemInfo() {
        StringBuilder sb = new StringBuilder();
        try (BufferedReader br = new BufferedReader(new FileReader("/proc/meminfo"))) {
            String line;
            while ((line = br.readLine()) != null) {
                sb.append(line).append("\n");
            }
        } catch (Exception ignored) {}
        return sb.toString();
    }

    private long readBatteryCurrentNow(BatteryManager bm) {
        long current = 0;
        if (bm != null) {
            try {
                current = bm.getLongProperty(BatteryManager.BATTERY_PROPERTY_CURRENT_NOW);
            } catch (Exception ignored) {}
        }
        if (current != 0 && current != Long.MIN_VALUE) {
            return current;
        }

        String[] sysfsPaths = new String[]{
                "/sys/class/power_supply/battery/current_now",
                "/sys/class/power_supply/bms/current_now",
                "/sys/class/power_supply/main/current_now"
        };

        for (String path : sysfsPaths) {
            try (BufferedReader br = new BufferedReader(new FileReader(path))) {
                String line = br.readLine();
                if (line != null) {
                    long val = Long.parseLong(line.trim());
                    if (val != 0) {
                        return val;
                    }
                }
            } catch (Exception ignored) {}
        }
        return 0L;
    }

    private void readCpuStatsAndFreqs(int[] usagePercents, long[] curFreqs) {
        try {
            Process p = Runtime.getRuntime().exec(new String[]{
                "su", "-c", "grep '^cpu[0-7]' /proc/stat; cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq"
            });
            BufferedReader br = new BufferedReader(new InputStreamReader(p.getInputStream()));
            String line;
            int freqIdx = 0;
            while ((line = br.readLine()) != null) {
                line = line.trim();
                if (line.isEmpty()) continue;

                if (line.startsWith("cpu")) {
                    CpuStatsParser.CpuStatSnapshot snapshot = CpuStatsParser.parseStatLine(line);
                    if (snapshot != null) {
                        int cpuId = snapshot.getCpuId();
                        if (cpuId >= 0 && cpuId < usagePercents.length) {
                            CpuStatsParser.CpuStatSnapshot prev = new CpuStatsParser.CpuStatSnapshot(
                                    cpuId, prevCpuTotal[cpuId], prevCpuIdle[cpuId]);
                            if (prevCpuTotal[cpuId] > 0) {
                                usagePercents[cpuId] = CpuStatsParser.calculateUsage(prev, snapshot);
                            } else {
                                usagePercents[cpuId] = 0;
                            }
                            prevCpuTotal[cpuId] = snapshot.getTotalTime();
                            prevCpuIdle[cpuId] = snapshot.getIdleTime();
                        }
                    }
                } else {
                    if (freqIdx < curFreqs.length) {
                        curFreqs[freqIdx] = CpuStatsParser.parseFreqLineToMhz(line);
                        freqIdx++;
                    }
                }
            }
            p.waitFor();
        } catch (Exception ignored) {}
    }
}
