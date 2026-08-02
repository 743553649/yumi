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

if echo "$CURRENT_LOCALE" | $BUSYBOX grep -qi "zh"; then
  LANG_CODE="zh"
  MSG_WELCOME="欢迎使用 Yumi 调度！"
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

# --- 结束 ---
# 保留模块默认配置不变，不进行任何文件操作