package com.yumi.bridge;

import com.yumi.bridge.model.AppRuleItem;
import com.yumi.bridge.model.RealLogEntry;
import com.yumi.bridge.apps.AppRulesManager;
import com.yumi.bridge.ipc.IpcClient;
import com.yumi.bridge.system.SystemDashboardMonitor;

import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.View;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Toast;

import androidx.compose.ui.platform.ComposeView;
import com.yumi.bridge.ui.compose.ComposeHomeBridgeKt;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
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

    private final List<RealLogEntry> realLogs = new ArrayList<>();

    private String currentMode = "balance";
    private int daemonPort = 14567;
    private int activeTab = TAB_HOME;

    // IPC 通信层（FIX-022 拆分自上帝类）
    private final IpcClient ipcClient = new IpcClient(daemonPort);

    private final Map<String, String> appModesMap = new LinkedHashMap<>();
    private final List<AppRuleItem> allAppItems = new ArrayList<>();

    // 应用规则管理器（FIX-022 拆分自上帝类），共享 appModesMap/allAppItems 引用，
    // 命令发送回调交回 MainActivity.sendCommand（复用其线程化 + 日志刷新编排）
    private final AppRulesManager appRulesManager =
            new AppRulesManager(this, appModesMap, allAppItems, this::sendCommand);

    private final Handler pollHandler = new Handler(Looper.getMainLooper());
    private final ExecutorService backgroundIoExecutor = Executors.newSingleThreadExecutor();
    private final Handler mainHandler = new Handler(Looper.getMainLooper());

    // 系统仪表盘采集器（FIX-022 拆分自上帝类），CPU 占用率依赖其内部 prevCpu 采样
    private final SystemDashboardMonitor systemDashboardMonitor =
            new SystemDashboardMonitor(this, mainHandler);

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
        appRulesManager.loadAppRulesFromYaml();

        // Attach Compose global backdrop background and main screen view
        if (composeBackgroundHost != null) {
            ComposeHomeBridgeKt.attachBackgroundHost(composeBackgroundHost);
        }
        ComposeHomeBridgeKt.attachMainScreen(
                composeContentHost,
                this::setGlobalMode,
                appRulesManager::onAppModeChanged,
                this::onTabSelected,
                this::onUserRefreshLogs,
                this::onUserClearLogs,
                this::onFilterLevelChanged
        );

        // Initialize IPC communication and fetch mode logs
        sendCommand("get_mode");
        fetchRealModuleLogs();

        onTabSelected(TAB_HOME);

        // 启动后台轮询（2 秒刷新日志与系统仪表盘）
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
            appRulesManager.loadInstalledAppsList();
        }
    }

    private void setGlobalMode(String mode) {
        if (currentMode.equalsIgnoreCase(mode)) return;
        currentMode = mode;
        sendCommand("set_mode " + mode);
    }

    // ==================== Log Polling & IPC Command ====================

    private boolean isLogClearedByUser = false;

    private void onFilterLevelChanged(int level) {
        this.currentFilterLevel = level;
    }

    private void onUserClearLogs() {
        isLogClearedByUser = true;
        realLogs.clear();
        ComposeHomeBridgeKt.clearLogState();

        ipcClient.clearDaemonLogFile();

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
                    fetched.addAll(IpcClient.buildDefaultReadyLogs());
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
