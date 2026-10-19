#!/usr/bin/env bash
# GPU 合成与 CPU 合成**逐字节**对拍（设计 §13.3；v2 修订 R1、R16）。GPU 后端的决定性判据。
#
#   cargo build --release && cargo build --release --examples   # gpu 在默认 feature 里
#   bash tools/gpu-parity.sh
#
# 判据：下面每一项的差异像素数都必须是 0（P1 阈值 = 0，写在 doc/引擎测试说明.md）。
# 退出码：0 全部一致；1 有不一致 / 有项失败；3 没有可用 GPU（不算通过）；4 没编 gpu feature。
#
# 防「全绿但没测到」（damage-check 第一版的教训）：
#   - 每个 GPU 运行的日志里必须有「🎨 渲染后端：GPU」，否则两边可能都悄悄跑成了 CPU；
#   - 连续帧用例开 MINI_GPU_VERIFY=1：每帧读回与 CPU 对拍，「GPU 逐帧校验：N 帧」的 N
#     要不小于请求帧数的 90% —— 只比最后一张快照，增量镜像中途的记账错误是测不到的；
#   - 整轮下来「合成模式 分层」与「合成模式 直通」都要出现过；
#   - 注入 tint（着色器让 R 通道 +1）时必须报不一致，证明比较链路是活的。
set -u
cd "$(dirname "$0")/.."
export MINI_FAKE_NOW="${MINI_FAKE_NOW:-2024-03-15}"
BIN=./target/release/mini-app-window
OUT=target/_gp
[ -x "$BIN" ] || { echo "先 cargo build --release"; exit 1; }

"$BIN" --backend-probe >/dev/null 2>&1
case $? in
    0) ;;
    4) echo "❌ 这次构建关掉了 gpu feature（--no-default-features）：用默认 feature 重新 cargo build --release"; exit 4 ;;
    *) echo "⚠️ GPU 未测：没有可用适配器（软件适配器不算）"; exit 3 ;;
esac

rm -rf "$OUT" target/mini-storage
mkdir -p "$OUT/logs"
total=0; same=0; FAIL=0
bad() { echo "  ✗ $*"; FAIL=1; }

# run <名字> <后端> <最少校验帧数(0=不校验)> <mini-app-window 参数…>：出图到 $OUT/<名字>/<后端>
run() {
    local name="$1" be="$2" want="$3"; shift 3
    local log="$OUT/logs/${name}_${be}.log"
    rm -rf target/mini-storage "$OUT/$name/$be"
    if [ "$be" = gpu ]; then
        MINI_BACKEND=gpu MINI_GPU_LOG=1 MINI_GPU_VERIFY=1 "$BIN" "$@" --snapshot "$OUT/$name/$be" >"$log" 2>&1
    else
        MINI_BACKEND=cpu "$BIN" "$@" --snapshot "$OUT/$name/$be" >"$log" 2>&1
    fi
    local code=$?
    if [ "$be" = gpu ]; then
        [ $code = 5 ] && { bad "${name}：$(grep 'GPU 校验不一致' "$log" | head -1)"; return 1; }
        grep -q "渲染后端：GPU" "$log" || { bad "${name}：没走 GPU（$(grep -m1 '渲染后端' "$log")）"; return 1; }
        grep -q "GPU 运行中回退 CPU" "$log" && { bad "${name}：运行中回退了（$(grep -m1 '运行中回退' "$log")）"; return 1; }
        local n; n=$(grep -o 'GPU 逐帧校验：[0-9]*' "$log" | grep -o '[0-9]*$' | tail -1)
        if [ "$want" -gt 0 ] && [ "${n:-0}" -lt $(( want * 9 / 10 )) ]; then
            bad "${name}：只校验了 ${n:-0} 帧（要 ≥ $(( want * 9 / 10 ))）"; return 1
        fi
    fi
    [ $code = 0 ] || { bad "$name/$be 退出码 ${code}（${log}）"; return 1; }
}

# cmp_pair <名字>：cpu 与 gpu 两份输出逐个 PNG 比较
cmp_pair() {
    local name="$1" f rel
    for f in $(cd "$OUT/$name/cpu" 2>/dev/null && find . -name rust.png | sort); do
        rel="${f#./}"; total=$((total+1))
        if cmp -s "$OUT/$name/cpu/$rel" "$OUT/$name/gpu/$rel"; then
            same=$((same+1))
        else
            bad "$name/$rel $(python3 tools/pixdiff.py "$OUT/$name/cpu/$rel" "$OUT/$name/gpu/$rel" 2>&1 | tail -1)"
        fi
    done
}

pair() { # <名字> <最少校验帧数> <参数…>：CPU 与 GPU 各跑一次，最终快照逐字节比较
    local name="$1" want="$2"; shift 2
    run "$name" cpu 0 "$@" && run "$name" gpu "$want" "$@" && cmp_pair "$name"
}

# 带墙上时钟的场景（拖动后的惯性 / 回弹、GIF 取帧、Loading 转圈、倒计时）：两次运行之间
# 状态本来就不一样（同一个二进制跑两次 --drag 实测差 24%），跨运行比 PNG 比的是时钟不是 GPU。
# 这类只在**同一次运行里**逐帧对拍：每帧 GPU 读回与同一份图层的 CPU 合成逐字节比较。
within() { # <名字> <最少校验帧数> <参数…>
    local name="$1" want="$2"; shift 2
    total=$((total+1))
    run "$name" gpu "$want" "$@" && { same=$((same+1)); echo "  ✓ ${name}：$(grep -o 'GPU 逐帧校验：.*' "$OUT/logs/${name}_gpu.log" | tail -1)"; }
}

pages_of() { python3 -c "import json;print('\n'.join(json.load(open('$1/app.json'))['pages']))"; }

echo "### 静态页（sample-app / news-app 全部页面，--settle 0 --time 2）"
for app in sample-app sample/news-app; do
    for p in $(pages_of "sample/${app#sample/}"); do
        pair "static_$(echo "$app$p" | tr '/' '_')" 1 "$app" --route "$p" --settle 0 --time 2
    done
done

echo "### 增量重绘（damage-check 的 5 个用例，--frames 30 同一次运行内逐帧校验）"
dm() { within "damage_$1" 30 sample-app --route "$2" --time 2 --eval "$3" --frames 30; }
dm pk   pages/components/components '__callPageMethod("onPickerChange",{"value":2})'
dm qty  pages/detail/detail          '__callPageMethod("onQtyPlus",{})'
dm slot pages/showcase/showcase      '__callPageMethod("onSlot",{"value":1})'
dm spec pages/detail/detail          '__callPageMethod("onOpenSpec",{})'
dm tab  pages/category/category      '__callPageMethod("onSelectCategory",{"index":2})'

echo "### 连续拖动（逐帧校验：滚动、跨条带、增量镜像）"
cargo run --release --example backend_bench -- --gen-only "$OUT/apps" >/dev/null 2>&1 \
    || bad "生成夹具失败（cargo build --release --examples）"
within drag_r1 120 sample-app --route pages/index/index --settle 2 \
    --eval 'getCurrentPages()[0].setData({showCoupon:false})' --drag 8x120
within drag_news 120 sample/news-app --route pages/home/home --settle 2 --drag 8x120
within drag_s1 300 "$OUT/apps/scroll_list" --route pages/p/p --settle 0.5 --drag 8x300
within drag_s6 120 "$OUT/apps/nested_scroll" --route pages/p/p --settle 0.5 --drag 8x120
within modal_s5 60 "$OUT/apps/scroll_list" --route pages/p/p --settle 0.5 \
    --eval "wx.showModal({title:'基准',content:'覆盖层直通帧'})" --frames 60

echo "### 交互路径（MINI_BACKEND=gpu 跑 interaction-check，对照 CPU 基线）"
if [ -d target/baseline_main_ia ]; then
    MINI_BACKEND=gpu MINI_GPU_VERIFY=1 bash tools/interaction-check.sh "$OUT/ia_gpu" target/baseline_main_ia \
        >"$OUT/logs/ia.log" 2>&1 || bad "interaction-check（GPU）：$(grep -E '✗|❌' "$OUT/logs/ia.log" | head -3 | tr '\n' ' ')"
    grep '^一致' "$OUT/logs/ia.log" | sed 's/^/  /'
else
    bad "缺 target/baseline_main_ia（先在 CPU 下 bash tools/interaction-check.sh target/baseline_main_ia）"
fi
echo "### tabBar 点击（MINI_BACKEND=gpu）"
MINI_BACKEND=gpu bash tools/tab-click-check.sh >"$OUT/logs/tab.log" 2>&1 || bad "tab-click-check（GPU）失败"
tail -1 "$OUT/logs/tab.log" | sed 's/^/  /'

echo "### 防空转"
grep -lq "合成模式 分层" "$OUT"/logs/*_gpu.log || bad "整轮没出现过「合成模式 分层」"
grep -lq "合成模式 直通" "$OUT"/logs/*_gpu.log || bad "整轮没出现过「合成模式 直通」"
grep -h "合成模式 分层" "$OUT"/logs/*_gpu.log >/dev/null && echo "  ✓ 分层模式出现过"
grep -h "合成模式 直通" "$OUT"/logs/*_gpu.log >/dev/null && echo "  ✓ 直通模式出现过"
rm -rf target/mini-storage
MINI_BACKEND=gpu MINI_GPU_VERIFY=1 MINI_GPU_FAULT=tint "$BIN" sample-app --route pages/profile/profile \
    --settle 0 --time 2 --snapshot "$OUT/tint" >"$OUT/logs/tint.log" 2>&1
if [ $? = 5 ] && grep -q "GPU 校验不一致" "$OUT/logs/tint.log"; then
    echo "  ✓ tint 注入被检出：$(grep -m1 'GPU 校验不一致' "$OUT/logs/tint.log")"
else
    bad "tint 注入没被检出 —— 比较链路空转"
fi

echo "### 回退（S1 夹具，没有定时器，结果确定）"
S1=("$OUT/apps/scroll_list" --route pages/p/p --settle 0.5)
run fb_ref cpu 0 "${S1[@]}" --frames 60
for f in init selftest; do
    rm -rf target/mini-storage "$OUT/fb_$f"
    MINI_BACKEND=gpu MINI_GPU_FAULT=$f "$BIN" "${S1[@]}" --frames 60 --snapshot "$OUT/fb_$f" >"$OUT/logs/fb_$f.log" 2>&1
    total=$((total+1))
    if grep -q "GPU 回退 CPU：" "$OUT/logs/fb_$f.log" && cmp -s "$OUT/fb_ref/cpu/pages/p/p/rust.png" "$OUT/fb_$f/pages/p/p/rust.png"; then
        same=$((same+1)); echo "  ✓ ${f}：回退 CPU，快照与 CPU 逐字节一致"
    else
        bad "${f}：没回退或快照不一致（$OUT/logs/fb_$f.log）"
    fi
done
rm -rf target/mini-storage "$OUT/fb_lost"
MINI_BACKEND=gpu MINI_GPU_FAULT=lost@30 "$BIN" "${S1[@]}" --frames 60 --snapshot "$OUT/fb_lost" >"$OUT/logs/fb_lost.log" 2>&1
total=$((total+1))
if [ $? = 0 ] && grep -q "GPU 运行中回退 CPU：" "$OUT/logs/fb_lost.log" && grep -q "设备丢失" "$OUT/logs/fb_lost.log" \
    && cmp -s "$OUT/fb_ref/cpu/pages/p/p/rust.png" "$OUT/fb_lost/pages/p/p/rust.png"; then
    same=$((same+1)); echo "  ✓ lost@30：$(grep -m1 '运行中回退' "$OUT/logs/fb_lost.log")，快照与 CPU 逐字节一致"
else
    bad "lost@30：没有运行中回退 / 原因不含「设备丢失」/ 快照不一致（$OUT/logs/fb_lost.log）"
fi

echo
echo "GPU 与 CPU 逐字节一致：$same/$total"
rm -rf target/mini-storage
if [ "$FAIL" = 0 ] && [ "$same" = "$total" ]; then
    rm -rf "$OUT"
    echo "✅ GPU 合成与 CPU 逐字节一致（阈值 0）"
else
    echo "❌ 有不一致或失败（日志留在 $OUT/logs）"; exit 1
fi
