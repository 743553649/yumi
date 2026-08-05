#!/system/bin/sh
#
# ########################################################################################
#   yumi 模块安装脚本
#   作者: yuki
# ########################################################################################

# --- 模块路径和工具 ---
# $MODPATH 是 Magisk 传入的模块安装路径

# --- 自动检测 BusyBox (保留以备将来可能使用，当前未使用) ---
if [ -x "/data/adb/magisk/busybox" ]; then
  BUSYBOX="/data/adb/magisk/busybox"
elif [ -x "/data/adb/ksu/bin/busybox" ]; then
  BUSYBOX="/data/adb/ksu/bin/busybox"
elif [ -x "/data/adb/ap/bin/busybox" ]; then
  BUSYBOX="/data/adb/ap/bin/busybox"
fi

# --- 语言定义 ---
CURRENT_LOCALE=$(/system/bin/getprop persist.sys.locale)
if [ -z "$CURRENT_LOCALE" ]; then
    CURRENT_LOCALE=$(/system/bin/getprop ro.product.locale)
fi

LANG_CODE="en"
MSG_WELCOME="Welcome to Yumi Scheduler!"

GREP_CMD="grep"
if [ -n "$BUSYBOX" ] && [ -x "$BUSYBOX" ]; then
  GREP_CMD="$BUSYBOX grep"
fi

if echo "$CURRENT_LOCALE" | $GREP_CMD -qi "zh"; then
  LANG_CODE="zh"
  MSG_WELCOME="欢迎使用 Yumi 调度！"
fi

# --- 备份现有用户配置 (rules.yaml & config.yaml) ---
BAK_DIR="${TMPDIR:-$MODPATH}"

if [ -f "/data/adb/modules/yumi/rules.yaml" ]; then
    ui_print "- 检测到已存在用户配置，保留 rules.yaml..."
    cp -f "/data/adb/modules/yumi/rules.yaml" "$BAK_DIR/user_rules.yaml.bak"
fi

if [ -f "/data/adb/modules/yumi/config/config.yaml" ]; then
    ui_print "- 检测到已存在用户配置，保留 config.yaml..."
    cp -f "/data/adb/modules/yumi/config/config.yaml" "$BAK_DIR/user_config.yaml.bak"
elif [ -f "/data/adb/modules/yumi/config.yaml" ]; then
    ui_print "- 检测到已存在用户配置，保留 config.yaml..."
    cp -f "/data/adb/modules/yumi/config.yaml" "$BAK_DIR/user_config.yaml.bak"
fi

# --- 自动安装控制 App ---
if [ -f "$MODPATH/yumi-bridge.apk" ]; then
    ui_print "正在自动安装 yumi Bridge 控制 App..."
    pm install -r "$MODPATH/yumi-bridge.apk" >/dev/null 2>&1
    if [ $? -eq 0 ]; then
        ui_print "✓ yumi Bridge 控制 App 安装成功！"
    else
        ui_print "! yumi Bridge 控制 App 自动安装跳过"
    fi
fi

# --- 恢复用户配置 (rules.yaml & config.yaml) ---
if [ -f "$BAK_DIR/user_rules.yaml.bak" ]; then
    cp -f "$BAK_DIR/user_rules.yaml.bak" "$MODPATH/rules.yaml"
    ui_print "- 用户应用规则配置 (rules.yaml) 已成功还原！"
    rm -f "$BAK_DIR/user_rules.yaml.bak"
fi

if [ -f "$BAK_DIR/user_config.yaml.bak" ]; then
    if [ -d "$MODPATH/config" ]; then
        cp -f "$BAK_DIR/user_config.yaml.bak" "$MODPATH/config/config.yaml"
    else
        cp -f "$BAK_DIR/user_config.yaml.bak" "$MODPATH/config.yaml"
    fi
    ui_print "- 用户全局配置已成功还原！"
    rm -f "$BAK_DIR/user_config.yaml.bak"
fi

# --- 结束 ---