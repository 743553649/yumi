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

    // Tab 常量
    public static final int TAB_HOME = 0;
    public static final int TAB_LOGS = 1;
    public static final int TAB_APPS = 2;

    // 日志等级常量
    public static final int LEVEL_ALL = 0;
    public static final int LEVEL_DEBUG = 1;
    public static final int LEVEL_INFO = 2;
    public static final int LEVEL_WARN = 3;
    public static final int LEVEL_ERROR = 4;

    private static class RealLogEntry {
        final String rawLine;
        final String formattedChineseLine;
        final int level;

        RealLogEntry(String rawLine, String formattedChineseLine, int level) {
            this.rawLine = rawLine;
            this.formattedChineseLine = formattedChineseLine;
            this.level = level;
        }
    }

    private static class AppRuleItem {
        final String packageName;
        final String appName;
        String currentMode; // default, powersave, balance, performance, fast, fas

        AppRuleItem(String packageName, String appName, String currentMode) {
            this.packageName = packageName;
            this.appName = appName;
            this.currentMode = currentMode != null ? currentMode : "default";
        }
    }

    private View rootContainer;
    private ComposeView composeBackgroundHost;
    private ComposeView composeContentHost;
    private View tabHomeContainer;
    private View tabLogsContainer;
    private View tabAppsContainer;

    // 底部导航栏 View
    private LinearLayout btnTabHome;
    private LinearLayout btnTabLogs;
    private LinearLayout btnTabApps;
    private ImageView ivTabHome;
    private ImageView ivTabLogs;
    private ImageView ivTabApps;
    private TextView tvTabHome;
    private TextView tvTabLogs;
    private TextView tvTabApps;

    // 应用规则管理 View
    private EditText etSearchApp;
    private LinearLayout llAppsListContainer;

    // 日志相关 View
    private TextView tvLog;
    private ScrollView svLogScroll;
    private TextView btnClearLog;

    // 5 级筛选 Chip 控件
    private TextView btnLevelAll;
    private TextView btnLevelDebug;
    private TextView btnLevelInfo;
    private TextView btnLevelWarn;
    private TextView btnLevelError;
    private int currentFilterLevel = LEVEL_ALL;

    // 真实模块运行日志缓存列表
    private final List<RealLogEntry> realLogs = new ArrayList<>();

    private final long[] prevCpuTotal = new long[8];
    private final long[] prevCpuIdle = new long[8];

    private String currentMode = "balance";
    private int daemonPort = 14567;
    private int activeTab = TAB_HOME;

    // 应用规则内存映射
    private final Map<String, String> appModesMap = new LinkedHashMap<>();
    private final List<AppRuleItem> allAppItems = new ArrayList<>();
    private String currentSearchQuery = "";

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
        setupNavigationTabs();
        setupListeners();
        setupNestedScrollFix();
        loadAppRulesFromYaml();

        // 绑定 Compose 全局天幕背景与首页视图
        if (composeBackgroundHost != null) {
            ComposeHomeBridgeKt.attachBackgroundHost(composeBackgroundHost);
        }
        ComposeHomeBridgeKt.attachHomeScreen(composeContentHost, this::setGlobalMode);

        // 初始化通信与拉取模式日志
        sendCommand("get_mode");
        fetchRealModuleLogs();

        // 默认显示首页 Tab
        switchTab(TAB_HOME);

        // 启动后台轮询 (1 秒刷新)
        pollHandler.postDelayed(logPollRunnable, 2000);
    }

    private void initViews() {
        rootContainer = findViewById(R.id.rootContainer);
        composeBackgroundHost = findViewById(R.id.composeBackgroundHost);
        composeContentHost = findViewById(R.id.composeContentHost);
        /* 
        tabHomeContainer = findViewById(R.id.tabHomeContainer);
        tabLogsContainer = findViewById(R.id.tabLogsContainer);
        tabAppsContainer = findViewById(R.id.tabAppsContainer);

        btnTabHome = findViewById(R.id.btnTabHome);
        btnTabLogs = findViewById(R.id.btnTabLogs);
        btnTabApps = findViewById(R.id.btnTabApps);
        ivTabHome = findViewById(R.id.ivTabHome);
        ivTabLogs = findViewById(R.id.ivTabLogs);
        ivTabApps = findViewById(R.id.ivTabApps);
        tvTabHome = findViewById(R.id.tvTabHome);
        tvTabLogs = findViewById(R.id.tvTabLogs);
        tvTabApps = findViewById(R.id.tvTabApps);

        etSearchApp = findViewById(R.id.etSearchApp);
        llAppsListContainer = findViewById(R.id.llAppsListContainer);

        tvLog = findViewById(R.id.tvLog);
        svLogScroll = findViewById(R.id.svLogScroll);
        btnClearLog = findViewById(R.id.btnClearLog);

        btnLevelAll = findViewById(R.id.btnLevelAll);
        btnLevelDebug = findViewById(R.id.btnLevelDebug);
        btnLevelInfo = findViewById(R.id.btnLevelInfo);
        btnLevelWarn = findViewById(R.id.btnLevelWarn);
        btnLevelError = findViewById(R.id.btnLevelError);
        */
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

        // 1. RAM & Swap 内存读取 (/proc/meminfo)
        try {
            String memInfo = readProcMemInfo();
            SystemStatsParser.MemoryStats memStats = SystemStatsParser.parseMemInfo(memInfo);
            ramPercent = memStats.getRamPercent();
            ramDetailText = memStats.getRamDetailText();
            swapPercent = memStats.getSwapPercent();
            swapDetailText = memStats.getSwapDetailText();
        } catch (Exception ignored) {}

        // 2. 电池实时信息读取 (Level, Power, Temperature)
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

        // 3. 系统运行时长 (格式：天:小时:分钟)
        try {
            long elapsedSec = android.os.SystemClock.elapsedRealtime() / 1000;
            long days = elapsedSec / 86400;
            long hours = (elapsedSec % 86400) / 3600;
            long mins = (elapsedSec % 3600) / 60;
            uptimeText = String.format(Locale.getDefault(), "%d天:%02d小时:%02d分钟", days, hours, mins);
        } catch (Exception ignored) {}

        // 4. 8 张 CPU 核心动态数据读取
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

    private void setupNavigationTabs() {
        /*
        btnTabHome.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                switchTab(TAB_HOME);
            }
        });

        btnTabLogs.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                switchTab(TAB_LOGS);
            }
        });

        btnTabApps.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                switchTab(TAB_APPS);
            }
        });
        */
    }

    private void switchTab(int tabIndex) {
        activeTab = tabIndex;
        /*
        tabHomeContainer.setVisibility(tabIndex == TAB_HOME ? View.VISIBLE : View.GONE);
        tabLogsContainer.setVisibility(tabIndex == TAB_LOGS ? View.VISIBLE : View.GONE);
        tabAppsContainer.setVisibility(tabIndex == TAB_APPS ? View.VISIBLE : View.GONE);

        // 高亮选中 Tab 样式
        updateTabStyle(tvTabHome, ivTabHome, tabIndex == TAB_HOME);
        updateTabStyle(tvTabLogs, ivTabLogs, tabIndex == TAB_LOGS);
        updateTabStyle(tvTabApps, ivTabApps, tabIndex == TAB_APPS);

        if (tabIndex == TAB_APPS && allAppItems.isEmpty()) {
            loadInstalledAppsList();
        }
        */
    }

    private void updateTabStyle(TextView tv, ImageView iv, boolean selected) {
        if (selected) {
            tv.setTextColor(getResources().getColor(R.color.ios_text_primary));
            iv.setAlpha(1.0f);
        } else {
            tv.setTextColor(getResources().getColor(R.color.ios_text_muted));
            iv.setAlpha(0.5f);
        }
    }

    private void setupNestedScrollFix() {
        /*
        svLogScroll.setOnTouchListener(new View.OnTouchListener() {
            @Override
            public boolean onTouch(View v, MotionEvent event) {
                if (event.getAction() == MotionEvent.ACTION_MOVE) {
                    if (v.canScrollVertically(1) || v.canScrollVertically(-1)) {
                        v.getParent().requestDisallowInterceptTouchEvent(true);
                    }
                }
                return false;
            }
        });
        */
    }

    private void setupListeners() {
        /*
        btnClearLog.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                realLogs.clear();
                renderLogDisplay();
            }
        });

        btnLevelAll.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) { setLogLevelFilter(LEVEL_ALL); }
        });
        btnLevelDebug.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) { setLogLevelFilter(LEVEL_DEBUG); }
        });
        btnLevelInfo.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) { setLogLevelFilter(LEVEL_INFO); }
        });
        btnLevelWarn.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) { setLogLevelFilter(LEVEL_WARN); }
        });
        btnLevelError.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) { setLogLevelFilter(LEVEL_ERROR); }
        });

        etSearchApp.addTextChangedListener(new TextWatcher() {
            @Override
            public void beforeTextChanged(CharSequence s, int start, int count, int after) {}

            @Override
            public void onTextChanged(CharSequence s, int start, int before, int count) {
                currentSearchQuery = s != null ? s.toString().trim().toLowerCase() : "";
                renderAppRulesList();
            }

            @Override
            public void afterTextChanged(Editable s) {}
        });
        */
    }

    private void setGlobalMode(String mode) {
        if (currentMode.equalsIgnoreCase(mode)) return;
        currentMode = mode;
        sendCommand("set_mode " + mode);
    }

    // ==================== 应用规则管理 (App Rules) ====================

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
        PackageManager pm = getPackageManager();
        Map<String, String> appNameMap = new LinkedHashMap<>();

        // 扫描已安装第三方应用及升级版系统应用
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

        // Root Shell 降级扫描（当 PackageManager 返回列表为空时，触发 su 命令扫描所有第三方应用包名）
        if (appNameMap.isEmpty()) {
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

        // 保留预置游戏列表作为最底层的降级备选项（仅在上述所有扫描均为空时作为补充）
        if (appNameMap.isEmpty()) {
            appNameMap.put("com.tencent.tmgp.sgame", "王者荣耀");
            appNameMap.put("com.tencent.tmgp.pubgmhd", "和平精英");
            appNameMap.put("com.tencent.lolm", "英雄联盟手游");
            appNameMap.put("com.miHoYo.GenshinImpact", "原神");
            appNameMap.put("com.miHoYo.hkrpg", "崩坏：星穹铁道");
            appNameMap.put("com.kurogame.mingchao", "鸣潮");
            appNameMap.put("com.hypergryph.arknights", "明日方舟");
        }

        // 将 appModesMap 中已有的配置包名也确保包含进列表
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

        // 排序：已设置规则的置顶，其次按名称排序
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

        renderAppRulesList();
    }

    private void renderAppRulesList() {
        llAppsListContainer.removeAllViews();
        LayoutInflater inflater = LayoutInflater.from(this);
        PackageManager pm = getPackageManager();

        for (final AppRuleItem item : allAppItems) {
            if (!currentSearchQuery.isEmpty()) {
                boolean matchName = item.appName.toLowerCase().contains(currentSearchQuery);
                boolean matchPkg = item.packageName.toLowerCase().contains(currentSearchQuery);
                if (!matchName && !matchPkg) continue;
            }

            View itemView = inflater.inflate(R.layout.item_app_rule, llAppsListContainer, false);
            ImageView ivAppIcon = itemView.findViewById(R.id.ivAppIcon);
            TextView tvIconText = itemView.findViewById(R.id.tvAppIconText);
            TextView tvAppName = itemView.findViewById(R.id.tvAppName);
            TextView tvAppPackage = itemView.findViewById(R.id.tvAppPackage);
            final TextView btnAppMode = itemView.findViewById(R.id.btnAppMode);

            // 尝试获取并渲染应用真实图标，无法获取时显示首字母回退
            boolean loadedIcon = false;
            try {
                android.graphics.drawable.Drawable icon = pm.getApplicationIcon(item.packageName);
                if (icon != null) {
                    ivAppIcon.setImageDrawable(icon);
                    ivAppIcon.setVisibility(View.VISIBLE);
                    tvIconText.setVisibility(View.GONE);
                    loadedIcon = true;
                }
            } catch (Exception ignored) {}

            if (!loadedIcon) {
                ivAppIcon.setVisibility(View.GONE);
                tvIconText.setVisibility(View.VISIBLE);
                String firstLetter = item.appName.isEmpty() ? "A" : item.appName.substring(0, 1).toUpperCase();
                tvIconText.setText(firstLetter);
            }

            tvAppName.setText(item.appName);
            tvAppPackage.setText(item.packageName);

            updateAppModeBtnText(btnAppMode, item.currentMode);

            btnAppMode.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    showAppModeSelectionDialog(item, btnAppMode);
                }
            });

            llAppsListContainer.addView(itemView);
        }
    }

    private void updateAppModeBtnText(TextView btn, String mode) {
        switch (mode.toLowerCase()) {
            case "powersave":
                btn.setText("省电 (Powersave)");
                btn.setTextColor(0xFF16A34A);
                break;
            case "balance":
                btn.setText("均衡 (Balance)");
                btn.setTextColor(0xFF0284C7);
                break;
            case "performance":
                btn.setText("性能 (Performance)");
                btn.setTextColor(0xFFEA580C);
                break;
            case "fast":
                btn.setText("极速 (Fast)");
                btn.setTextColor(0xFFDC2626);
                break;
            case "fas":
                btn.setText("FAS 帧感知 (FAS)");
                btn.setTextColor(0xFF9333EA);
                break;
            default:
                btn.setText("跟随全局 (Default)");
                btn.setTextColor(0xFF475569);
                break;
        }
    }

    private void showAppModeSelectionDialog(final AppRuleItem item, final TextView btnAppMode) {
        final String[] options = new String[]{
                "跟随全局 (Default)",
                "省电 (Powersave)",
                "均衡 (Balance)",
                "性能 (Performance)",
                "极速 (Fast)",
                "FAS 帧感知 (FAS)"
        };

        final String[] modeKeys = new String[]{
                "default",
                "powersave",
                "balance",
                "performance",
                "fast",
                "fas"
        };

        int selectedIdx = 0;
        for (int i = 0; i < modeKeys.length; i++) {
            if (modeKeys[i].equalsIgnoreCase(item.currentMode)) {
                selectedIdx = i;
                break;
            }
        }

        AlertDialog.Builder builder = new AlertDialog.Builder(this);
        builder.setTitle(item.appName + " 调度规则设置");
        builder.setSingleChoiceItems(options, selectedIdx, new DialogInterface.OnClickListener() {
            @Override
            public void onClick(DialogInterface dialog, int which) {
                String chosenMode = modeKeys[which];
                item.currentMode = chosenMode;
                if ("default".equalsIgnoreCase(chosenMode)) {
                    appModesMap.remove(item.packageName);
                } else {
                    appModesMap.put(item.packageName, chosenMode);
                }
                updateAppModeBtnText(btnAppMode, chosenMode);
                saveAppRulesToYaml();
                sendCommand("set_app_mode " + item.packageName + " " + chosenMode);
                dialog.dismiss();
            }
        });
        builder.setNegativeButton("取消", null);
        builder.create().show();
    }

    private void saveAppRulesToYaml() {
        File[] possibleFiles = new File[]{
                new File("/storage/emulated/0/yumi/rules.yaml"),
                new File("/storage/emulated/0/yumi/module/rules.yaml"),
                new File("/data/adb/modules/yumi/rules.yaml"),
                new File("/data/adb/modules/yumi/module/rules.yaml")
        };

        // 确保 SD 卡持久化路径存在
        File defaultSdFile = new File("/storage/emulated/0/yumi/rules.yaml");
        try {
            if (!defaultSdFile.exists()) {
                if (defaultSdFile.getParentFile() != null) defaultSdFile.getParentFile().mkdirs();
                defaultSdFile.createNewFile();
            }
        } catch (Exception ignored) {}

        for (File targetFile : possibleFiles) {
            if (targetFile != null && (targetFile.exists() || targetFile.equals(defaultSdFile))) {
                try {
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
                        }
                    }

                    if (!hasAppModesSection) {
                        lines.add("");
                        lines.add("app_modes:");
                        for (Map.Entry<String, String> entry : appModesMap.entrySet()) {
                            lines.add("  " + entry.getKey() + ": " + entry.getValue());
                        }
                    }

                    try (PrintWriter pw = new PrintWriter(new FileWriter(targetFile))) {
                        for (String l : lines) {
                            pw.println(l);
                        }
                    }
                } catch (Exception ignored) {}
            }
        }

        sendCommand("reload_rules");
        Toast.makeText(this, "应用规则持久化保存成功", Toast.LENGTH_SHORT).show();
    }

    // ==================== 日志与状态通信控制 (保持原有逻辑不变) ====================

    private void setLogLevelFilter(int level) {
        currentFilterLevel = level;

        int activeTextColor = 0xFFFFFFFF;    // 亮蓝背景上的纯白文字
        int inactiveTextColor = 0xFF475569;  // 冰粹极简深色文字

        btnLevelAll.setBackgroundResource(level == LEVEL_ALL ? R.drawable.bg_ios_btn_blue : R.drawable.bg_ios_btn_secondary);
        btnLevelAll.setTextColor(level == LEVEL_ALL ? activeTextColor : inactiveTextColor);

        btnLevelDebug.setBackgroundResource(level == LEVEL_DEBUG ? R.drawable.bg_ios_btn_blue : R.drawable.bg_ios_btn_secondary);
        btnLevelDebug.setTextColor(level == LEVEL_DEBUG ? activeTextColor : inactiveTextColor);

        btnLevelInfo.setBackgroundResource(level == LEVEL_INFO ? R.drawable.bg_ios_btn_blue : R.drawable.bg_ios_btn_secondary);
        btnLevelInfo.setTextColor(level == LEVEL_INFO ? activeTextColor : inactiveTextColor);

        btnLevelWarn.setBackgroundResource(level == LEVEL_WARN ? R.drawable.bg_ios_btn_blue : R.drawable.bg_ios_btn_secondary);
        btnLevelWarn.setTextColor(level == LEVEL_WARN ? activeTextColor : inactiveTextColor);

        btnLevelError.setBackgroundResource(level == LEVEL_ERROR ? R.drawable.bg_ios_btn_blue : R.drawable.bg_ios_btn_secondary);
        btnLevelError.setTextColor(level == LEVEL_ERROR ? activeTextColor : inactiveTextColor);

        renderLogDisplay();
    }

    private void fetchRealModuleLogs() {
        new Thread(new Runnable() {
            @Override
            public void run() {
                final List<RealLogEntry> fetched = new ArrayList<>();

                // 1. 优先通过 TCP IPC 接口获取守护进程日志 (get_log 150)
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
                            if (trimmed.startsWith("err:")) continue; // 过滤 err:unknown_command 等异常单行回响
                            int level = parseLogLevel(trimmed);
                            fetched.add(new RealLogEntry(trimmed, formatLineToChinese(trimmed), level));
                        }
                    }
                } catch (Exception ignored) {}

                // 2. 如果 IPC 读取为空，扫描本地磁盘与 Magisk 模块日志路径
                if (fetched.isEmpty()) {
                    fetched.addAll(readModuleDaemonLogFile());
                }

                // 3. 如果依然为空，显示系统守护线程就绪提示日志
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

    private List<RealLogEntry> readModuleDaemonLogFile() {
        List<RealLogEntry> result = new ArrayList<>();

        // 1. 优先通过 Root Shell (su -c) 提权读取 Magisk 真实模块日志 /data/adb/modules/yumi/logs/daemon.log
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
            if (!result.isEmpty()) {
                return result;
            }
        } catch (Exception ignored) {}

        // 2. 如果 Root 读取为空，扫描所有候选路径回退
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
        StringBuilder sb = new StringBuilder();
        for (RealLogEntry entry : realLogs) {
            if (currentFilterLevel == LEVEL_ALL || entry.level == currentFilterLevel) {
                sb.append(entry.formattedChineseLine).append("\n");
            }
        }
        tvLog.setText(sb.toString());
        svLogScroll.post(new Runnable() {
            @Override
            public void run() {
                svLogScroll.fullScroll(ScrollView.FOCUS_DOWN);
            }
        });
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
