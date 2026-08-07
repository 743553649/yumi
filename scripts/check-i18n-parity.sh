#!/usr/bin/env bash
#
# i18n 三端 en/zh 键集对等校验
#
# 设计前提（FIX-026 重新定义目标）：
#   三端用途不同、key 集合按设计就不可能跨端相等——
#     - fluent  (守护进程日志)   en.ftl / zh.ftl       各 124 键
#     - WebUI   (Web UI 标签)    en.ts  / zh.ts         各 69 键
#     - Android (原生 UI 标签)   values/ (zh 默认) / values-en/ (en)  各 46 键
#   故只校验「各端内部 en/zh 键集对等」，不做跨端 key 相等校验，
#   也不做以 .ftl 生成 strings.xml / ts（技术不成立）。
#
# 退出码：0 = 全部对等；1 = 有缺失/多余键。
#
set -euo pipefail

# 脚本按仓库根目录为工作目录运行（CI 中 checkout 后默认在仓库根）。
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
FAIL=0

# ---- 提取函数：输出已排序、去重的 key 列表到 stdout ----

# fluent .ftl：行首 identifier = （排除缩进的续行与 # 注释）
extract_fluent() {
    { grep -E '^[a-zA-Z0-9_-]+\s*=' "$1" || true; } \
        | sed -E 's/^([a-zA-Z0-9_-]+)\s*=.*/\1/' | sort -u
}

# WebUI .ts：缩进的 key: （扁平结构，无嵌套对象）
extract_webui() {
    { grep -E '^[[:space:]]+[a-zA-Z0-9_]+:' "$1" || true; } \
        | sed -E 's/^[[:space:]]+([a-zA-Z0-9_]+):.*/\1/' | sort -u
}

# Android strings.xml：<string name="key">
extract_android() {
    { grep -oE '<string[[:space:]]+name="[a-zA-Z0-9_]+"' "$1" || true; } \
        | sed -E 's/.*name="([a-zA-Z0-9_]+)".*/\1/' | sort -u
}

# ---- 比对函数 ----
check() {
    local label="$1" en_file="$2" zh_file="$3"
    local en_n zh_n only_en only_zh
    en_n=$(wc -l < "$en_file" | tr -d ' ')
    zh_n=$(wc -l < "$zh_file" | tr -d ' ')
    only_en=$(comm -23 "$en_file" "$zh_file" || true)
    only_zh=$(comm -13 "$en_file" "$zh_file" || true)
    if [ -n "$only_en" ] || [ -n "$only_zh" ]; then
        echo "FAIL: $label en/zh 键集不一致 (en=$en_n, zh=$zh_n)"
        [ -n "$only_en" ] && { echo "  仅 en 有:"; echo "$only_en" | sed 's/^/    /'; }
        [ -n "$only_zh" ] && { echo "  仅 zh 有:"; echo "$only_zh" | sed 's/^/    /'; }
        FAIL=1
    else
        echo "OK:   $label en/zh 键集对等 ($en_n keys)"
    fi
}

# ---- 三端校验 ----

extract_fluent "module/config/i18n/en.ftl" > "$TMPDIR/fluent_en"
extract_fluent "module/config/i18n/zh.ftl" > "$TMPDIR/fluent_zh"
check "fluent  " "$TMPDIR/fluent_en" "$TMPDIR/fluent_zh"

extract_webui "webui/src/i18n/locales/en.ts" > "$TMPDIR/webui_en"
extract_webui "webui/src/i18n/locales/zh.ts" > "$TMPDIR/webui_zh"
check "WebUI  " "$TMPDIR/webui_en" "$TMPDIR/webui_zh"

echo ""
if [ "$FAIL" -ne 0 ]; then
    echo "i18n 对等校验未通过：请补齐缺失键或删除多余键。"
    exit 1
fi
echo "全部三端 i18n en/zh 键集对等校验通过。"
