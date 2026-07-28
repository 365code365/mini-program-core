#!/usr/bin/env bash
# 底部 tabBar 点击回归：三个示例小程序 x 每个 tab，逐个点过去看是否切到正确页面。
#
# 必须用 `--touch` 而不是 `--click`：`--click` 直接调 handle_click，绕过整个指针层，
# 而真机（窗体的 MouseInput）走的是 on_pointer_press/release。曾经就是因为只用
# `--click` 测，漏掉了「按下 tabBar 时提前 return，触摸状态机没启动 -> 抬手没有 tap
# -> handle_click 永远不被调用」这条 bug：无头全绿，真机上 tabBar 完全点不动。
#
# 用法: bash tools/tab-click-check.sh
set -u
BIN=./target/release/mini-app-window
if [ ! -x "$BIN" ]; then echo "先 cargo build --release"; exit 1; fi

fail=0

# 参数: app_root 起始路由 点击y "x:期望页面" ...
check() {
    app="$1"; route="$2"; y="$3"; shift 3
    echo "### $app (从 $route 出发, 点击 y=$y)"
    for spec in "$@"; do
        x="${spec%%:*}"; want="${spec##*:}"
        rm -rf target/mini-storage
        got=$("$BIN" "$app" --route "$route" --settle 1 --time 2 \
                --touch "$x,$y" --snapshot target/_tabchk 2>&1 |
              grep -oE 'pages/[a-z/-]+/rust.png' | tail -1 | sed 's|/rust.png||')
        if [ "$got" = "$want" ]; then
            printf '  OK   x=%-4s -> %s\n' "$x" "$got"
        else
            printf '  FAIL x=%-4s -> %s (期望 %s)\n' "$x" "${got:-无}" "$want"
            fail=$(expr "$fail" + 1)
        fi
    done
}

# sample-app: app.json 声明 tabBar + custom-tab-bar 自定义渲染, 4 个 tab, 条高 50
check sample-app pages/index/index 642 \
    47:pages/index/index 141:pages/category/category \
    234:pages/cart/cart 328:pages/profile/profile

# news-app: 同上, 3 个 tab, 条高 49
check sample/news-app pages/home/home 645 \
    62:pages/home/home 187:pages/video/video 312:pages/mine/mine

# tea-app: tabBar 是页面里的 usingComponents 组件(app.json 没有 tabBar),
# 走页面事件冒泡那条链路, 一并守住
if [ -f sample/tea-app/app.json ]; then
    check sample/tea-app pages/index/index 640 \
        37:pages/index/index 112:pages/choose/choose \
        187:pages/gift/gift 262:pages/origin/origin 337:pages/mine/mine
fi

rm -rf target/_tabchk target/mini-storage
if [ "$fail" = 0 ]; then echo "全部通过 OK"; else echo "$fail 项失败 FAIL"; exit 1; fi
