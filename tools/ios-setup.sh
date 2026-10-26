#!/usr/bin/env bash
# 检查 / 补齐 iOS 构建与调试环境（可反复跑，已装的跳过）。
#
#   bash tools/ios-setup.sh          # 检查 + 装能自动装的（rust 目标、brew 工具、flutter iOS 产物）
#   bash tools/ios-setup.sh --check  # 只检查，不装
#
# 能自动装的：rust 的 aarch64-apple-ios / aarch64-apple-ios-sim、xcodes / aria2 / xcbeautify / ios-deploy
# （brew）、CocoaPods、Flutter 的 iOS 引擎产物。
# **装不了、要人来的**：Xcode 本体（要 Apple ID）。三条路任选：
#   1. App Store 搜 Xcode 安装（最省事；装完跑一次本脚本）
#   2. `xcodes install --latest --experimental-unxip`（终端里会问 Apple ID 与双重认证码；装了 aria2 会多线程下载）
#   3. developer.apple.com/download 下 .xip，`xcodes install --path <xip>`
# 装完之后本脚本负责：xcode-select 指过去、接受许可、首启组件、下载 iOS 模拟器运行时。
#
# 为什么只有命令行工具不够：`cargo build --target aarch64-apple-ios` 要 iPhoneOS SDK，
# `xcodebuild -create-xcframework`、`simctl`、真机安装都在 Xcode 里，CommandLineTools 一个都没有
# （报 `SDK "iphoneos" cannot be located`）。
#
# 占盘：Xcode ~12GB 下载、~40GB 解开；iOS 模拟器运行时 ~8GB。开装前看一眼 `df -h /`。
set -u
cd "$(dirname "$0")/.."
CHECK=0
[ "${1:-}" = "--check" ] && CHECK=1
. "$HOME/.cargo/env" 2>/dev/null || true
export HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK=1 HOMEBREW_NO_ENV_HINTS=1
MISSING=0
ok()   { echo "✅ $*"; }
miss() { echo "❌ $*"; MISSING=$((MISSING + 1)); }
note() { echo "   $*"; }

# ── Xcode ──────────────────────────────────────────────────────────────
XCODE=""
for app in /Applications/Xcode.app /Applications/Xcode-*.app /Applications/Xcode_*.app; do
    [ -d "$app/Contents/Developer" ] && { XCODE="$app"; break; }
done
if [ -z "$XCODE" ]; then
    miss "没有 Xcode（只有 $(xcode-select -p 2>/dev/null || echo 无)）"
    note "装法见本脚本文件头：App Store 搜 Xcode，或 xcodes install --latest --experimental-unxip"
    note "空闲盘：$(df -h / | awk 'NR==2{print $4}')（Xcode 解开约 40GB，另加模拟器运行时约 8GB）"
else
    ok "Xcode：$XCODE（$("$XCODE/Contents/Developer/usr/bin/xcodebuild" -version 2>/dev/null | head -1)）"
    # sudo 一律 `-n`：本脚本也会被 ios-debug.sh 在后台调起，没有终端时交互式 sudo 会一直挂着等密码
    if [ "${DEVELOPER_DIR:-}" = "$XCODE/Contents/Developer" ]; then
        ok "DEVELOPER_DIR 已指向 Xcode（不动 xcode-select）"
    elif [ "$(xcode-select -p 2>/dev/null)" != "$XCODE/Contents/Developer" ]; then
        if [ "$CHECK" = 0 ] && [ -t 0 ]; then
            echo "▶ sudo xcode-select -s $XCODE（要输开机密码）"
            sudo xcode-select -s "$XCODE/Contents/Developer"
        elif [ "$CHECK" = 0 ] && sudo -n xcode-select -s "$XCODE/Contents/Developer" 2>/dev/null; then
            ok "xcode-select 已切到 $XCODE"
        else
            export DEVELOPER_DIR="$XCODE/Contents/Developer"
            note "xcode-select 还指着 $(xcode-select -p)；本次用 DEVELOPER_DIR（想永久切：sudo xcode-select -s $XCODE）"
        fi
    fi
    if ! xcodebuild -license check >/dev/null 2>&1 || ! xcodebuild -checkFirstLaunchStatus >/dev/null 2>&1; then
        if [ "$CHECK" = 0 ] && [ -t 0 ]; then
            echo "▶ 接受许可并装首启组件（要输开机密码）"
            sudo xcodebuild -license accept && sudo xcodebuild -runFirstLaunch
        else
            miss "Xcode 许可 / 首启组件未完成（打开一次 Xcode 点同意，或终端里跑本脚本）"
        fi
    fi
    if xcrun --sdk iphoneos --show-sdk-version >/dev/null 2>&1; then
        ok "iPhoneOS SDK $(xcrun --sdk iphoneos --show-sdk-version)，模拟器 SDK $(xcrun --sdk iphonesimulator --show-sdk-version 2>/dev/null)"
    else
        miss "xcrun 找不到 iphoneos SDK"
    fi
    if xcrun simctl list runtimes 2>/dev/null | grep -q '^iOS'; then
        ok "模拟器运行时：$(xcrun simctl list runtimes | grep '^iOS' | tail -1 | cut -c1-60)"
    elif [ "$CHECK" = 1 ]; then
        miss "没有 iOS 模拟器运行时"
    else
        echo "▶ 下载 iOS 模拟器运行时（约 8GB）"
        xcodebuild -downloadPlatform iOS || miss "模拟器运行时下载失败（Xcode → Settings → Components 手动装）"
    fi
fi

# ── Rust 目标 ───────────────────────────────────────────────────────────
for t in aarch64-apple-ios aarch64-apple-ios-sim; do
    if rustup target list --installed 2>/dev/null | grep -qx "$t"; then
        ok "rust 目标 $t"
    elif [ "$CHECK" = 1 ]; then
        miss "rust 目标 $t"
    else
        rustup target add "$t"
    fi
done

# ── brew 工具 ───────────────────────────────────────────────────────────
# xcodes：命令行装 / 切 Xcode；aria2：给 xcodes 多线程下载；xcbeautify：xcodebuild 日志压成可读的几行；
# ios-deploy：命令行把 .app 装上真机并挂 lldb（不开 Xcode 界面调真机）
for f in xcodes aria2 xcbeautify ios-deploy; do
    bin=$f; [ "$f" = aria2 ] && bin=aria2c
    if command -v "$bin" >/dev/null; then
        ok "$f"
    elif [ "$CHECK" = 1 ]; then
        miss "$f（brew install $f）"
    else
        brew install --formula "$f" >/dev/null && ok "$f（刚装）" || miss "$f 安装失败"
    fi
done

# ── CocoaPods / Flutter（Flutter 插件示例 sdk/flutter/example 用） ─────────
command -v pod >/dev/null && ok "CocoaPods $(pod --version 2>/dev/null)" || miss "CocoaPods（brew install cocoapods）"
if command -v flutter >/dev/null; then
    if [ -n "$XCODE" ] && [ "$CHECK" = 0 ]; then
        flutter precache --ios >/dev/null 2>&1 && ok "Flutter iOS 引擎产物" || miss "flutter precache --ios 失败"
    else
        ok "flutter（iOS 产物要 Xcode 装好后 flutter precache --ios）"
    fi
fi

echo
if [ "$MISSING" = 0 ]; then
    echo "iOS 环境齐了。下一步：bash tools/build-mobile.sh ios"
else
    echo "还缺 $MISSING 项（见上面 ❌）。装好后再跑一次 bash tools/ios-setup.sh"
fi
[ "$MISSING" = 0 ]
