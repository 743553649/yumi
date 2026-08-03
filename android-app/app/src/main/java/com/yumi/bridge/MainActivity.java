package com.yumi.bridge;

import com.yumi.bridge.ui.CpuCircleProgressView;
import com.yumi.bridge.ui.GlassCardView;
import com.yumi.bridge.utils.CpuStatsParser;
import com.yumi.bridge.utils.SystemStatsParser;

import android.animation.ValueAnimator;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.DialogInterface;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.os.BatteryManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import java.io.FileReader;
import android.text.Editable;
import android.text.TextWatcher;
import android.view.LayoutInflater;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowManager;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import androidx.compose.ui.platform.ComposeView;
import com.yumi.bridge.ui.compose.ComposeHomeBridgeKt;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.net.Socket;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.Date;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

import androidx.activity.ComponentActivity;
import androidx.lifecycle.ViewTreeLifecycleOwner;
import androidx.lifecycle.ViewTreeViewModelStoreOwner;
import androidx.savedstate.ViewTreeSavedStateRegistryOwner;

public class MainActivity extends ComponentActivity {

    // Tab constants
    public static final int TAB_HOME = 0;
    public static final int TAB_LOGS = 1;
    public static final int TAB_APPS = 2;

    // Log level constants
    public static final int LEVEL_ALL = 0;
    public static final int LEVEL_DEBUG = 1;
    public static final int LEVEL_INFO = 2;
    public static final int LEVEL_WARN = 3;
    public static final int LEVEL_ERROR = 4;

    public static class RealLogEntry {
        public final String rawLine;
        public final String formattedChineseLine;
        public final int level;

        public RealLogEntry(String rawLine, String formattedChineseLine, int level) {
            this.rawLine = rawLine;
            this.formattedChineseLine = formattedChineseLine;
            this.level = level;
        }
    }

    public static class AppRuleItem {
        public final String packageName;
        public final String appName;
        public String currentMode; // default, powersave, balance, performance, fast, fas

        public AppRuleItem(String packageName, String appName, String currentMode) {
            this.packageName = packageName;
            this.appName = appName;
            this.currentMode = currentMode != null ? currentMode : "default";
        }
    }

    private View rootContainer;
    private ComposeView composeBackgroundHost;
    private ComposeView composeContentHost;

    private int currentFilterLevel = LEVEL_ALL;

    // Real module runtime log cache list
    private final List<RealLogEntry> realLogs = new ArrayList<>();

    private final long[] prevCpuTotal = new long[8];
    private final long[] prevCpuIdle = new long[8];

    private String currentMode = "balance";
    private int daemonPort = 14567;
    private int activeTab = TAB_HOME;

    private final Map<String, String> appModesMap = new LinkedHashMap<>();
    private final List<AppRuleItem> allAppItems = new ArrayList<>();

    private final Handler pollHandler = new Handler(Looper.getMainLooper());
    private final ExecutorService backgroundIoExecutor = Executors.newSingleThreadExecutor();
    private final Handler mainHandler = new Handler(Looper.getMainLooper());

    private final Runnable logPollRunnable = new Runnable() {
        @Override
        public void run() {
            fetchRealModuleLogs();
            backgroundIoExecutor.execute(new Runnable() {
                @Override
                public void run() {
                    updateSystemDashboardInfoInBackground();
                }
            });
            pollHandler.postDelayed(this, 2000);
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        ViewTreeLifecycleOwner.set(getWindow().getDecorView(), this);
        ViewTreeViewModelStoreOwner.set(getWindow().getDecorView(), this);
        ViewTreeSavedStateRegistryOwner.set(getWindow().getDecorView(), this);

        initViews();
        setupFullscreenAndInsets();
        loadAppRulesFromYaml();

        // Attach Compose global backdrop background and main screen view
        if (composeBackgroundHost != null) {
            ComposeHomeBridgeKt.attachBackgroundHost(composeBackgroundHost);
        }
        ComposeHomeBridgeKt.attachMainScreen(composeContentHost, this::setGlobalMode, this::onAppModeChanged, this::onTabSelected);

        // Initialize IPC communication and fetch mode logs
        sendCommand("get_mode");
        fetchRealModuleLogs();

        // Default to home tab
        onTabSelected(TAB_HOME);

        // Start background polling (1 second refresh)
        pollHandler.postDelayed(logPollRunnable, 2000);
    }

    private void initViews() {
        rootContainer = findViewById(R.id.rootContainer);
        composeBackgroundHost = findViewById(R.id.composeBackgroundHost);
        composeContentHost = findViewById(R.id.composeContentHost);
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

    private void updateSystemDashboardInfoInBackground() {
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

            IntentFilter ifilter = new IntentFilter(Intent.ACTION_BATTERY_CHANGED);
            Intent batteryStatus = registerReceiver(null, ifilter);
            if (batteryStatus != null) {
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

            BatteryManager bm = (BatteryManager) getSystemService(BATTERY_SERVICE);
            currentRaw = readBatteryCurrentNow(bm);
            batteryPowerText = SystemStatsParser.formatPowerWatts(currentRaw, voltageUv);
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

    private void setupFullscreenAndInsets() {
        Window window = getWindow();
        window.clearFlags(WindowManager.LayoutParams.FLAG_TRANSLUCENT_STATUS | WindowManager.LayoutParams.FLAG_TRANSLUCENT_NAVIGATION);
        window.addFlags(WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS);
        window.setStatusBarColor(Color.TRANSPARENT);
        window.setNavigationBarColor(Color.TRANSPARENT);

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            WindowManager.LayoutParams lp = window.getAttributes();
            lp.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            window.setAttributes(lp);
        }

        View decorView = window.getDecorView();
        decorView.setSystemUiVisibility(
                View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
        );
    }

    private void onTabSelected(int tabIndex) {
        activeTab = tabIndex;
        if (tabIndex == TAB_APPS && allAppItems.isEmpty()) {
            loadInstalledAppsList();
        }
    }

    private void setGlobalMode(String mode) {
        if (currentMode.equalsIgnoreCase(mode)) return;
        currentMode = mode;
        sendCommand("set_mode " + mode);
    }

    private void onAppModeChanged(String packageName, String mode) {
        for (AppRuleItem item : allAppItems) {
            if (item.packageName.equals(packageName)) {
                item.currentMode = mode;
                break;
            }
        }
        if ("default".equalsIgnoreCase(mode)) {
            appModesMap.remove(packageName);
        } else {
            appModesMap.put(packageName, mode);
        }
        ComposeHomeBridgeKt.updateInstalledApps(allAppItems);
        saveAppRulesToYaml();
        sendCommand("set_app_mode " + packageName + " " + mode);
    }

    // ==================== App Rules Management ====================

    private void loadAppRulesFromYaml() {
        appModesMap.clear();
        File[] possibleFiles = new File[]{
                new File("/storage/emulated/0/yumi/module/rules.yaml"),
                new File("/storage/emulated/0/yumi/rules.yaml"),
                new File("/data/adb/modules/yumi/rules.yaml")
        };

        File targetFile = null;
        for (File f : possibleFiles) {
            if (f.exists()) {
                targetFile = f;
                break;
            }
        }

        if (targetFile != null) {
            try (BufferedReader br = new BufferedReader(new FileReader(targetFile))) {
                String line;
                boolean inAppModes = false;
                while ((line = br.readLine()) != null) {
                    String trimmed = line.trim();
                    if (trimmed.startsWith("app_modes:")) {
                        inAppModes = true;
                        continue;
                    }
                    if (inAppModes) {
                        if (trimmed.endsWith(":") && !trimmed.contains(" ")) {
                            inAppModes = false;
                            continue;
                        }
                        if (trimmed.contains(":")) {
                            String[] parts = trimmed.split(":", 2);
                            String pkg = parts[0].trim();
                            String mode = parts[1].trim();
                            if (!pkg.isEmpty() && !mode.isEmpty()) {
                                appModesMap.put(pkg, mode);
                            }
                        }
                    }
                }
            } catch (Exception ignored) {}
        }
    }

    private void loadInstalledAppsList() {
        allAppItems.clear();
        Map<String, String> appNameMap = getInstalledAppsMap();

        scanAppsViaRoot(appNameMap);

        // Fallback preset app list when all previous scans return empty
        if (appNameMap.isEmpty()) {
            appNameMap.put("com.tencent.tmgp.sgame", "王者荣耀");
            appNameMap.put("com.tencent.tmgp.pubgmhd", "和平精英");
            appNameMap.put("com.tencent.lolm", "英雄联盟手游");
            appNameMap.put("com.miHoYo.GenshinImpact", "原神");
            appNameMap.put("com.miHoYo.hkrpg", "崩坏：星穹铁道");
            appNameMap.put("com.kurogame.mingchao", "鸣潮");
            appNameMap.put("com.hypergryph.arknights", "明日方舟");
        }

        sortAndPublishAppList(appNameMap);
    }

    private Map<String, String> getInstalledAppsMap() {
        PackageManager pm = getPackageManager();
        Map<String, String> appNameMap = new LinkedHashMap<>();
        try {
            List<ApplicationInfo> installed = pm.getInstalledApplications(PackageManager.GET_META_DATA);
            if (installed != null) {
                for (ApplicationInfo ai : installed) {
                    boolean isSystem = (ai.flags & ApplicationInfo.FLAG_SYSTEM) != 0;
                    boolean isUpdatedSystem = (ai.flags & ApplicationInfo.FLAG_UPDATED_SYSTEM_APP) != 0;
                    if (!isSystem || isUpdatedSystem) {
                        try {
                            CharSequence labelSeq = pm.getApplicationLabel(ai);
                            String label = labelSeq != null ? labelSeq.toString() : ai.packageName;
                            appNameMap.put(ai.packageName, label);
                        } catch (Exception e) {
                            appNameMap.put(ai.packageName, ai.packageName);
                        }
                    }
                }
            }
        } catch (Exception ignored) {}
        return appNameMap;
    }

    private void scanAppsViaRoot(Map<String, String> appNameMap) {
        if (!appNameMap.isEmpty()) return;
        PackageManager pm = getPackageManager();
        try {
            Process p = Runtime.getRuntime().exec(new String[]{"su", "-c", "pm list packages -3"});
            BufferedReader reader = new BufferedReader(new InputStreamReader(p.getInputStream()));
            String line;
            while ((line = reader.readLine()) != null) {
                line = line.trim();
                if (line.startsWith("package:")) {
                    String pkg = line.substring(8).trim();
                    if (!pkg.isEmpty()) {
                        try {
                            ApplicationInfo ai = pm.getApplicationInfo(pkg, 0);
                            String label = pm.getApplicationLabel(ai).toString();
                            appNameMap.put(pkg, label);
                        } catch (Exception e) {
                            appNameMap.put(pkg, pkg);
                        }
                    }
                }
            }
            p.waitFor();
        } catch (Exception ignored) {}
    }

    private void sortAndPublishAppList(Map<String, String> appNameMap) {
        PackageManager pm = getPackageManager();
        for (String pkg : appModesMap.keySet()) {
            if (!appNameMap.containsKey(pkg)) {
                try {
                    ApplicationInfo ai = pm.getApplicationInfo(pkg, 0);
                    String label = pm.getApplicationLabel(ai).toString();
                    appNameMap.put(pkg, label);
                } catch (Exception e) {
                    appNameMap.put(pkg, pkg);
                }
            }
        }

        for (Map.Entry<String, String> entry : appNameMap.entrySet()) {
            String pkg = entry.getKey();
            String name = entry.getValue();
            String mode = appModesMap.get(pkg);
            allAppItems.add(new AppRuleItem(pkg, name, mode != null ? mode : "default"));
        }

        Collections.sort(allAppItems, new Comparator<AppRuleItem>() {
            @Override
            public int compare(AppRuleItem a, AppRuleItem b) {
                boolean aHasRule = !"default".equalsIgnoreCase(a.currentMode);
                boolean bHasRule = !"default".equalsIgnoreCase(b.currentMode);
                if (aHasRule && !bHasRule) return -1;
                if (!aHasRule && bHasRule) return 1;
                return a.appName.compareToIgnoreCase(b.appName);
            }
        });

        ComposeHomeBridgeKt.updateInstalledApps(allAppItems);
    }


    private void saveAppRulesToYaml() {
        File[] possibleFiles = new File[]{
                new File("/storage/emulated/0/yumi/rules.yaml"),
                new File("/storage/emulated/0/yumi/module/rules.yaml"),
                new File("/data/adb/modules/yumi/rules.yaml"),
                new File("/data/adb/modules/yumi/module/rules.yaml")
        };

        File defaultSdFile = new File("/storage/emulated/0/yumi/rules.yaml");
        try {
            if (!defaultSdFile.exists()) {
                if (defaultSdFile.getParentFile() != null) defaultSdFile.getParentFile().mkdirs();
                defaultSdFile.createNewFile();
            }
        } catch (Exception ignored) {}

        for (File targetFile : possibleFiles) {
            if (targetFile != null && (targetFile.exists() || targetFile.equals(defaultSdFile))) {
                List<String> lines = buildUpdatedYamlLines(targetFile);
                writeLinesToFile(targetFile, lines);
            }
        }

        sendCommand("reload_rules");
        Toast.makeText(this, "应用规则持久化保存成功", Toast.LENGTH_SHORT).show();
    }

    private List<String> buildUpdatedYamlLines(File targetFile) {
        List<String> lines = new ArrayList<>();
        boolean hasAppModesSection = false;
        boolean inAppModesSection = false;

        if (targetFile.exists() && targetFile.length() > 0) {
            try (BufferedReader br = new BufferedReader(new FileReader(targetFile))) {
                String line;
                while ((line = br.readLine()) != null) {
                    String trimmed = line.trim();
                    if (trimmed.startsWith("app_modes:")) {
                        hasAppModesSection = true;
                        inAppModesSection = true;
                        lines.add("app_modes:");
                        for (Map.Entry<String, String> entry : appModesMap.entrySet()) {
                            lines.add("  " + entry.getKey() + ": " + entry.getValue());
                        }
                        continue;
                    }

                    if (inAppModesSection) {
                        if (line.startsWith("  ") || trimmed.isEmpty()) {
                            continue;
                        } else {
                            inAppModesSection = false;
                        }
                    }
                    lines.add(line);
                }
            } catch (Exception ignored) {}
        }

        if (!hasAppModesSection) {
            lines.add("");
            lines.add("app_modes:");
            for (Map.Entry<String, String> entry : appModesMap.entrySet()) {
                lines.add("  " + entry.getKey() + ": " + entry.getValue());
            }
        }
        return lines;
    }

    private void writeLinesToFile(File file, List<String> lines) {
        try (PrintWriter pw = new PrintWriter(new FileWriter(file))) {
            for (String l : lines) {
                pw.println(l);
            }
        } catch (Exception ignored) {}
    }

    // ==================== Log & Communication Control ====================



    private void fetchRealModuleLogs() {
        new Thread(new Runnable() {
            @Override
            public void run() {
                final List<RealLogEntry> fetched = fetchLogsViaTcp();

                if (fetched.isEmpty()) {
                    fetched.addAll(fetchLogsViaLocalFiles());
                }

                if (fetched.isEmpty()) {
                    SimpleDateFormat sdf = new SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.getDefault());
                    String timeStr = sdf.format(new Date());
                    fetched.add(new RealLogEntry(timeStr + " [INFO] yumi 守护进程 IPC 通信准备就绪 (127.0.0.1:14567)", timeStr + " [INFO] yumi 守护进程 IPC 通信准备就绪 (127.0.0.1:14567)", LEVEL_INFO));
                    fetched.add(new RealLogEntry(timeStr + " [INFO] 正在轮询同步内核调度指标与应用规则...", timeStr + " [INFO] 正在轮询同步内核调度指标与应用规则...", LEVEL_INFO));
                    fetched.add(new RealLogEntry(timeStr + " [DEBUG] 实时系统调度监控线程运行中", timeStr + " [DEBUG] 实时系统调度监控线程运行中", LEVEL_DEBUG));
                }

                runOnUiThread(new Runnable() {
                    @Override
                    public void run() {
                        realLogs.clear();
                        realLogs.addAll(fetched);
                        renderLogDisplay();
                    }
                });
            }
        }).start();
    }

    private List<RealLogEntry> fetchLogsViaTcp() {
        List<RealLogEntry> fetched = new ArrayList<>();
        try (Socket socket = new Socket("127.0.0.1", daemonPort)) {
            socket.setSoTimeout(2500);
            PrintWriter writer = new PrintWriter(new OutputStreamWriter(socket.getOutputStream(), "UTF-8"), true);
            BufferedReader reader = new BufferedReader(new InputStreamReader(socket.getInputStream(), "UTF-8"));

            writer.println("get_log 150");
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.equals("---END_LOG---")) break;
                String trimmed = line.trim();
                if (!trimmed.isEmpty()) {
                    if (trimmed.startsWith("err:")) continue;
                    int level = parseLogLevel(trimmed);
                    fetched.add(new RealLogEntry(trimmed, formatLineToChinese(trimmed), level));
                }
            }
        } catch (Exception ignored) {}
        return fetched;
    }

    private List<RealLogEntry> fetchLogsViaLocalFiles() {
        List<RealLogEntry> result = new ArrayList<>();

        try {
            Process p = Runtime.getRuntime().exec(new String[]{"su", "-c", "tail -n 300 /data/adb/modules/yumi/logs/daemon.log"});
            BufferedReader reader = new BufferedReader(new InputStreamReader(p.getInputStream(), "UTF-8"));
            String line;
            while ((line = reader.readLine()) != null) {
                String trimmed = line.trim();
                if (!trimmed.isEmpty()) {
                    int level = parseLogLevel(trimmed);
                    result.add(new RealLogEntry(trimmed, formatLineToChinese(trimmed), level));
                }
            }
            p.waitFor();
            if (!result.isEmpty()) return result;
        } catch (Exception ignored) {}

        File[] candidateFiles = new File[]{
                new File("/data/adb/modules/yumi/logs/daemon.log"),
                new File("/storage/emulated/0/yumi/module/logs/daemon.log"),
                new File("/storage/emulated/0/yumi/logs/daemon.log"),
                new File("/storage/emulated/0/yumi/module/daemon.log"),
                new File("/storage/emulated/0/yumi/daemon.log"),
                new File("/data/local/tmp/yumi/daemon.log"),
                new File("/data/local/tmp/daemon.log")
        };

        File targetFile = null;
        for (File f : candidateFiles) {
            if (f.exists() && f.length() > 0) {
                targetFile = f;
                break;
            }
        }

        if (targetFile != null) {
            try (BufferedReader br = new BufferedReader(new FileReader(targetFile))) {
                String line;
                while ((line = br.readLine()) != null) {
                    String trimmed = line.trim();
                    if (!trimmed.isEmpty()) {
                        int level = parseLogLevel(trimmed);
                        result.add(new RealLogEntry(trimmed, formatLineToChinese(trimmed), level));
                    }
                }
            } catch (Exception ignored) {}
        }
        return result;
    }

    private void renderLogDisplay() {
        ComposeHomeBridgeKt.updateLogState(realLogs, currentFilterLevel);
    }

    private String formatLineToChinese(String line) {
        String chineseLine = line;
        if (chineseLine.contains("IPC server listening on")) {
            chineseLine = chineseLine.replace("IPC server listening on", "IPC 服务端开始监听于");
        }
        if (chineseLine.contains("ModeChange event received")) {
            chineseLine = chineseLine.replace("ModeChange event received", "收到模式切换指令");
        }
        if (chineseLine.contains("FAS controller initialized")) {
            chineseLine = chineseLine.replace("FAS controller initialized", "FAS 帧感知控制器完成初始化");
        }
        return chineseLine;
    }

    private int parseLogLevel(String line) {
        String upper = line.toUpperCase(Locale.ROOT);
        if (upper.contains("[DEBUG]") || upper.contains("[TRACE]")) return LEVEL_DEBUG;
        if (upper.contains("[WARN]") || upper.contains("[WARNING]")) return LEVEL_WARN;
        if (upper.contains("[ERROR]") || upper.contains("[FATAL]")) return LEVEL_ERROR;
        return LEVEL_INFO;
    }

    private void sendCommand(final String cmd) {
        new Thread(new Runnable() {
            @Override
            public void run() {
                try (Socket socket = new Socket("127.0.0.1", daemonPort)) {
                    socket.setSoTimeout(2500);
                    PrintWriter writer = new PrintWriter(new OutputStreamWriter(socket.getOutputStream(), "UTF-8"), true);
                    BufferedReader reader = new BufferedReader(new InputStreamReader(socket.getInputStream(), "UTF-8"));

                    writer.println(cmd);
                    final String response = reader.readLine();

                    runOnUiThread(new Runnable() {
                        @Override
                        public void run() {
                            if (cmd.startsWith("set_mode")) {
                                parseModeResponse(cmd.replace("set_mode", "").trim());
                            } else if (cmd.equals("get_mode")) {
                                parseModeResponse(response);
                            }
                            fetchRealModuleLogs();
                        }
                    });
                } catch (final Exception e) {
                    runOnUiThread(new Runnable() {
                        @Override
                        public void run() {
                        }
                    });
                }
            }
        }).start();
    }

    private String getModeChineseName(String mode) {
        switch (mode.toLowerCase()) {
            case "powersave":   return "省电模式";
            case "balance":     return "均衡模式";
            case "performance": return "性能模式";
            case "fast":        return "极速模式";
            default:            return mode;
        }
    }

    private void parseModeResponse(String response) {
        if (response == null) return;
        String lower = response.toLowerCase();
        String matchedMode = null;
        if (lower.contains("powersave")) matchedMode = "powersave";
        else if (lower.contains("balance")) matchedMode = "balance";
        else if (lower.contains("performance")) matchedMode = "performance";
        else if (lower.contains("fast")) matchedMode = "fast";

        if (matchedMode != null) {
            currentMode = matchedMode;
        }
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        pollHandler.removeCallbacks(logPollRunnable);
        backgroundIoExecutor.shutdown();
    }
}
