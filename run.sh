#!/bin/bash
# Mini App 运行脚本
#
# 用法：
#   ./run.sh                 交互式选择 sample/ 下的小程序
#   ./run.sh sample-app      直接跑指定小程序（裸名字、sample/xxx、任意路径都行）
#   ./run.sh 2               按菜单编号直接跑
#   ./run.sh --list          只列出可用的小程序
#   ./run.sh --debug ...     用 debug 模式编译
#   ./run.sh --clean         先 cargo clean
#   ./run.sh --fps ...       开启帧率诊断（MINI_FPS=1）
#
# 示例小程序统一收在 sample/ 下，含 app.json 的一层子目录即视为一个小程序。

set -euo pipefail
cd "$(dirname "$0")"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
DIM='\033[2m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Mini App Engine Runner         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════╝${NC}"
echo ""

# macOS 上 bindgen 需要
export LIBCLANG_PATH="${LIBCLANG_PATH:-/Library/Developer/CommandLineTools/usr/lib}"

SAMPLE_DIR="sample"
MODE="--release"
APP_ARG=""
LIST_ONLY=0
FPS=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --debug) MODE=""; echo -e "${GREEN}▶ Debug mode${NC}"; shift ;;
        --clean) echo -e "${GREEN}▶ Cleaning...${NC}"; cargo clean; shift ;;
        --list|-l) LIST_ONLY=1; shift ;;
        --fps) FPS=1; shift ;;
        -h|--help) sed -n '3,14p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) APP_ARG="$1"; shift ;;
    esac
done

# ── 扫描 sample/ 下的小程序（含 app.json 的一层子目录）──
# 只看一层，所以 sample/_archive/... 这类归档不会被当成可运行示例。
APPS=()
if [[ -d "$SAMPLE_DIR" ]]; then
    while IFS= read -r d; do
        APPS+=("$(basename "$d")")
    done < <(find "$SAMPLE_DIR" -mindepth 1 -maxdepth 1 -type d | sort | while read -r d; do
        [[ -f "$d/app.json" ]] && echo "$d"
    done)
fi

if [[ ${#APPS[@]} -eq 0 ]]; then
    echo -e "${RED}✗ 在 $SAMPLE_DIR/ 下没有找到小程序（需要包含 app.json）${NC}"
    exit 1
fi

# 每个小程序显示页数，便于分辨
describe() {
    local name="$1" dir="$SAMPLE_DIR/$1" pages=0
    if [[ -f "$dir/app.json" ]]; then
        pages=$(grep -oE '"pages/[^"]+"' "$dir/app.json" 2>/dev/null | sort -u | wc -l | tr -d ' ')
    fi
    printf "%s ${DIM}(%s 页)${NC}" "$name" "$pages"
}

list_apps() {
    echo -e "${GREEN}可用的小程序：${NC}"
    local i=1
    for a in "${APPS[@]}"; do
        printf "  ${YELLOW}%d${NC}) %b\n" "$i" "$(describe "$a")"
        i=$((i + 1))
    done
}

if [[ $LIST_ONLY -eq 1 ]]; then
    list_apps
    exit 0
fi

# ── 决定要跑哪个 ──
CHOICE=""
if [[ -n "$APP_ARG" ]]; then
    CHOICE="$APP_ARG"
else
    list_apps
    echo ""
    # 非交互环境（CI、管道）直接用第一个，不要卡住等输入
    if [[ ! -t 0 ]]; then
        CHOICE="${APPS[0]}"
        echo -e "${DIM}非交互环境，默认选择 ${CHOICE}${NC}"
    else
        read -r -p "$(echo -e "请选择编号或直接输入名字 ${DIM}[默认 1]${NC}: ")" CHOICE || true
        CHOICE="${CHOICE:-1}"
    fi
fi

# 纯数字 -> 按菜单编号取名字
if [[ "$CHOICE" =~ ^[0-9]+$ ]]; then
    idx=$((CHOICE - 1))
    if [[ $idx -lt 0 || $idx -ge ${#APPS[@]} ]]; then
        echo -e "${RED}✗ 编号超出范围：${CHOICE}（共 ${#APPS[@]} 个）${NC}"
        exit 1
    fi
    CHOICE="${APPS[$idx]}"
fi

echo ""
echo -e "${GREEN}▶ 加载小程序: ${YELLOW}${CHOICE}${NC}"
[[ $FPS -eq 1 ]] && echo -e "${DIM}  帧率诊断已开启（MINI_FPS=1）${NC}"
echo ""

# 名字会由引擎侧的 app_dir::resolve 解析（裸名字 / sample/xxx / 任意路径都支持）
if [[ $FPS -eq 1 ]]; then
    MINI_FPS=1 cargo run $MODE --bin mini-app-window -- "$CHOICE"
else
    cargo run $MODE --bin mini-app-window -- "$CHOICE"
fi
