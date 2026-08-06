package com.yumi.bridge.model;

/**
 * 单个应用的调度规则条目数据模型。
 * 从 MainActivity 上帝类中拆出（FIX-022）。
 */
public final class AppRuleItem {
    public final String packageName;
    public final String appName;
    public String currentMode; // default, powersave, balance, performance, fast, fas

    public AppRuleItem(String packageName, String appName, String currentMode) {
        this.packageName = packageName;
        this.appName = appName;
        this.currentMode = currentMode != null ? currentMode : "default";
    }
}
