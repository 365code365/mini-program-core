#!/bin/bash

# Mini App 运行脚本 - 支持传入小程序目录

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Mini App Engine Runner         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════╝${NC}"
echo ""

# 设置 LIBCLANG_PATH (macOS)
export LIBCLANG_PATH="/Library/Developer/CommandLineTools/usr/lib"

# 默认使用 release 模式
MODE="--release"
APP_DIR=""

# 解析参数
while [[ $# -gt 0 ]]; do
    case $1 in
        --debug)
            MODE=""
            echo -e "${GREEN}▶ Debug mode${NC}"
            shift
            ;;
        --clean)
            echo -e "${GREEN}▶ Cleaning...${NC}"
            cargo clean
            shift
            ;;
        *)
            # 非选项参数视为小程序目录
            APP_DIR="$1"
            shift
            ;;
    esac
done

if [[ -n "$APP_DIR" ]]; then
    echo -e "${GREEN}▶ 加载小程序: ${YELLOW}$APP_DIR${NC}"
    LIBCLANG_PATH="/Library/Developer/CommandLineTools/usr/lib" cargo run $MODE --bin mini-app-window -- "$APP_DIR"
else
    echo -e "${GREEN}▶ 使用内置 sample-app${NC}"
    LIBCLANG_PATH="/Library/Developer/CommandLineTools/usr/lib" cargo run $MODE --bin mini-app-window
fi
