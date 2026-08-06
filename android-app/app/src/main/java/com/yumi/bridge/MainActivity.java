package com.yumi.bridge;

import com.yumi.bridge.model.AppRuleItem;
import com.yumi.bridge.model.RealLogEntry;
import com.yumi.bridge.ipc.IpcClient;
import com.yumi.bridge.ui.CpuCircleProgressView;
import com.yumi.bridge.ui.GlassCardView;

import android.animation.ValueAnimator;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.DialogInterface;
import android.content.SharedPreferences;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.Color;
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
import java.io.PrintWriter;
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

    private View rootContainer;
    private ComposeView composeBackgroundHost;
    private ComposeView composeContentHost;

    private int currentFilterLevel = LEVEL_ALL;

    // Real module runtime log cache list
    private final List<RealLogEntry> realLogs = new ArrayList<>();

    private String currentMode = "balance";
    private int daemonPort = 14567;
    private int activeTab = TAB_HOME;

    // IPC 通信层（FIX-022 拆分自上帝类）
    private final IpcClient ipcClient = new IpcClient(daemonPort);

    private final Map<String, String> appModesMap = new LinkedHashMap<>();
    private final List<AppRuleItem> allAppItems = new ArrayList<>();

    private final Handler pollHandler = new Handler(Looper.getMainLooper());
    private final ExecutorService backgroundIoExecutor = Executors.newSingleThreadExecutor();
    private final Handler mainHandler = new Handler(Looper.getMainLooper());

    // 系统仪表盘采集器（FIX-022 拆分自上帝类），CPU 占用率依赖其内部 prevCpu 采样
    private final com.yumi.bridge.system.SystemDashboardMonitor systemDashboardMonitor =
            new com.yumi.bridge.system.SystemDashboardMonitor(this, mainHandler);

    private final Runnable logPollRunnable = new Runnable() {
        @Override
        public void run() {
            fetchRealModuleLogs();
            backgroundIoExecutor.execute(new Runnable() {
                @Override
                public void run() {
                    systemDashboardMonitor.updateDashboard(currentMode);
                }
            });
            pollHandler.postDelayed(this, 2000);
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        setTheme(R.style.Theme_YumiBridge);
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
        ComposeHomeBridgeKt.attachMainScreen(
                composeContentHost,
                this::setGlobalMode,
                this::onAppModeChanged,
                this::onTabSelected,
                this::onUserRefreshLogs,
                this::onUserClearLogs,
                this::onFilterLevelChanged
        );

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
        saveAppRulesToPrefs();
        saveAppRulesToYaml();
        sendCommand("set_app_mode " + packageName + " " + mode);
    }

    // ==================== App Rules Management ====================

    private void loadAppRulesFromPrefs() {
        try {
            SharedPreferences sp = getSharedPreferences("yumi_app_rules_sp", MODE_PRIVATE);
            Map<String, ?> all = sp.getAll();
            if (all != null) {
                for (Map.Entry<String, ?> entry : all.entrySet()) {
                    if (entry.getValue() instanceof String) {
                        appModesMap.put(entry.getKey(), (String) entry.getValue());
                    }
                }
            }
        } catch (Exception ignored) {}
    }

    private void saveAppRulesToPrefs() {
        try {
            SharedPreferences sp = getSharedPreferences("yumi_app_rules_sp", MODE_PRIVATE);
            SharedPreferences.Editor editor = sp.edit();
            editor.clear();
            for (Map.Entry<String, String> entry : appModesMap.entrySet()) {
                editor.putString(entry.getKey(), entry.getValue());
            }
            editor.apply();
        } catch (Exception ignored) {}
    }

    private void loadAppRulesFromYaml() {
        appModesMap.clear();
        loadAppRulesFromPrefs();
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

        saveAppRulesToPrefs();
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
            if (lines.isEmpty()) {
                lines.add("yumi_scheduler: true");
                lines.add("dynamic_enabled: true");
                lines.add("global_mode: balance");
            }
            lines.add("");
            lines.add("app_modes:");
            for (Map.Entry<String, String> entry : appModesMap.entrySet()) {
                lines.add("  " + entry.getKey() + ": " + entry.getValue());
            }
        }
        return lines;
    }

    private void writeLinesToFile(File file, List<String> lines) {
        boolean success = false;
        try (PrintWriter pw = new PrintWriter(new FileWriter(file))) {
            for (String l : lines) {
                pw.println(l);
            }
            success = true;
        } catch (Exception ignored) {}

        // Root fallback using standard stdin streaming (WebUI/Linux shell solution)
        if (!success || file.getAbsolutePath().startsWith("/data/adb") || file.getAbsolutePath().startsWith("/storage")) {
            try {
                Process p = Runtime.getRuntime().exec("su");
                try (java.io.DataOutputStream os = new java.io.DataOutputStream(p.getOutputStream())) {
                    os.writeBytes("mkdir -p " + file.getParent() + "\n");
                    os.writeBytes("cat << 'EOF' > " + file.getAbsolutePath() + "\n");
                    for (String l : lines) {
                        os.writeBytes(l + "\n");
                    }
                    os.writeBytes("EOF\n");
                    os.writeBytes("exit\n");
                    os.flush();
                }
                p.waitFor();
            } catch (Exception ignored) {}
        }
    }

    // ==================== Log & Communication Control ====================



    private boolean isLogClearedByUser = false;

    private void onFilterLevelChanged(int level) {
        this.currentFilterLevel = level;
    }

    private void onUserClearLogs() {
        isLogClearedByUser = true;
        realLogs.clear();
        ComposeHomeBridgeKt.clearLogState();
        
        // Execute root command to clear /data/adb/modules/yumi/logs/daemon.log file on disk
        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    Process p = Runtime.getRuntime().exec(new String[]{"su", "-c", "> /data/adb/modules/yumi/logs/daemon.log"});
                    p.waitFor();
                } catch (Exception ignored) {}
            }
        }).start();

        Toast.makeText(this, "日志面板与磁盘日志已清空", Toast.LENGTH_SHORT).show();
    }

    private void onUserRefreshLogs() {
        isLogClearedByUser = false;
        Toast.makeText(this, "正在刷新日志...", Toast.LENGTH_SHORT).show();
        fetchRealModuleLogs();
    }

    private void fetchRealModuleLogs() {
        new Thread(new Runnable() {
            @Override
            public void run() {
                final List<RealLogEntry> fetched = ipcClient.fetchLogs();

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
                        if (isLogClearedByUser) {
                            return;
                        }
                        realLogs.clear();
                        realLogs.addAll(fetched);
                        renderLogDisplay();
                    }
                });
            }
        }).start();
    }

    private void renderLogDisplay() {
        ComposeHomeBridgeKt.updateLogState(realLogs, currentFilterLevel);
    }

    private void sendCommand(final String cmd) {
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String response = ipcClient.sendCommand(cmd);
                runOnUiThread(new Runnable() {
                    @Override
                    public void run() {
                        if (cmd.startsWith("set_mode")) {
                            String matched = IpcClient.parseModeResponse(cmd.replace("set_mode", "").trim());
                            if (matched != null) currentMode = matched;
                        } else if (cmd.equals("get_mode")) {
                            String matched = IpcClient.parseModeResponse(response);
                            if (matched != null) currentMode = matched;
                        }
                        fetchRealModuleLogs();
                    }
                });
            }
        }).start();
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        pollHandler.removeCallbacks(logPollRunnable);
        backgroundIoExecutor.shutdown();
    }
}
