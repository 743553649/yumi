package com.yumi.bridge.apps;

import com.yumi.bridge.model.AppRuleItem;
import com.yumi.bridge.ui.compose.ComposeHomeBridgeKt;

import android.content.Context;
import android.content.SharedPreferences;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.widget.Toast;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.util.Collections;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * 应用调度规则管理器。从 MainActivity 上帝类中拆出（FIX-022）。
 *
 * 职责：维护 appModesMap（包名→模式）与 allAppItems（列表项）两份共享状态，
 * 负责 SharedPreferences 读写、已装应用列表扫描与排序、模式变更后的持久化
 * 与守护进程通知。rules.yaml 文件读写委托给 RulesYamlStore。
 *
 * 与外部的协作：
 * - appModesMap / allAppItems 由 MainActivity 注入并共享引用（双向可见）。
 * - 命令发送通过 CommandSender 回调交回 MainActivity，复用其线程+UI+日志刷新编排。
 * - 列表变更后直接调用 ComposeHomeBridgeKt.updateInstalledApps 推送到 Compose。
 */
public class AppRulesManager {

    /** 命令发送回调：交回 MainActivity 走其 sendCommand（含线程化与日志刷新）。 */
    public interface CommandSender {
        void send(String cmd);
    }

    private final Context context;
    private final Map<String, String> appModesMap;
    private final List<AppRuleItem> allAppItems;
    private final CommandSender commandSender;
    private final RulesYamlStore rulesYamlStore = new RulesYamlStore();

    public AppRulesManager(Context context,
                           Map<String, String> appModesMap,
                           List<AppRuleItem> allAppItems,
                           CommandSender commandSender) {
        this.context = context;
        this.appModesMap = appModesMap;
        this.allAppItems = allAppItems;
        this.commandSender = commandSender;
    }

    // ==================== App Rules Management ====================

    public void loadAppRulesFromPrefs() {
        try {
            SharedPreferences sp = context.getSharedPreferences("yumi_app_rules_sp", Context.MODE_PRIVATE);
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

    public void saveAppRulesToPrefs() {
        try {
            SharedPreferences sp = context.getSharedPreferences("yumi_app_rules_sp", Context.MODE_PRIVATE);
            SharedPreferences.Editor editor = sp.edit();
            editor.clear();
            for (Map.Entry<String, String> entry : appModesMap.entrySet()) {
                editor.putString(entry.getKey(), entry.getValue());
            }
            editor.apply();
        } catch (Exception ignored) {}
    }

    public void loadAppRulesFromYaml() {
        appModesMap.clear();
        loadAppRulesFromPrefs();
        rulesYamlStore.mergeAppModesFromYaml(appModesMap);
    }

    public void loadInstalledAppsList() {
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
        PackageManager pm = context.getPackageManager();
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
        PackageManager pm = context.getPackageManager();
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
        PackageManager pm = context.getPackageManager();
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


    public void saveAppRulesToYaml() {
        rulesYamlStore.writeAppModesToYaml(appModesMap);
        saveAppRulesToPrefs();
        commandSender.send("reload_rules");
        Toast.makeText(context, "应用规则持久化保存成功", Toast.LENGTH_SHORT).show();
    }

    // ==================== Mode Change ====================

    public void onAppModeChanged(String packageName, String mode) {
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
        commandSender.send("set_app_mode " + packageName + " " + mode);
    }
}
