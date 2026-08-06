package com.yumi.bridge.model

/**
 * 单个应用的调度规则条目数据模型。
 * 从 MainActivity 上帝类中拆出（FIX-022）。
 *
 * 字段以 @JvmField 暴露为公开字段，保留原 Java 调用方（AppRulesManager 等）
 * 的直接字段访问/写入语义；待存量 Java 渐进 Kotlin 化后可移除 @JvmField。
 */
class AppRuleItem(
    packageName: String,
    appName: String,
    currentMode: String?
) {
    @JvmField
    val packageName: String = packageName

    @JvmField
    val appName: String = appName

    @JvmField
    var currentMode: String = currentMode ?: "default"
}
