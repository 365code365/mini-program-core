#!/usr/bin/env bash
# 交互路径的回归：`snapshot-all.sh` 只出静态页、`damage-check.sh` 只走 --eval，
# 两者都绕过**指针层与覆盖层**（picker 面板、Modal 按钮、按压态、手势仲裁）。
# 重构这两层时，这里是唯一的判据。
#
#   bash tools/interaction-check.sh                         # 出图到 target/baseline_ia
#   bash tools/interaction-check.sh <目录A> <目录B>          # 出图到 A 并与基线 B 逐像素比
#
# 基线放在 `target/baseline_*` 下：`clean-target.sh` 对这个前缀留了白名单，
# 不会被回归后的清理带走 —— 被带走的话下一次对比只能报「基线缺图」，
# 而重新生成的基线已经是改动**之后**的了，参照物就永远丢了（这轮踩了两次）。
#
# ## 哪些能比像素、哪些不能
# 拖动惯性与 CSS/面板的入场动画按**真实时间**推进，同一条命令跑两次结果本来就不同
# （实测同一个二进制两次 `--drag 8x30` 差 24% 像素）。所以：
#   · 能比像素的场景一律「等动画走完再截图」，并避开带每秒 setData 的页面
#     （首页秒杀倒计时会让任何真实等待的快照都差几十个像素）；
#   · 手势归属这类**时间相关**的行为改用日志断言（MINI_SCROLL_LOG），比像素稳。
set -u
# 快照回归统一用**固定日期**：有页面用 `new Date()` 高亮当天（news-app 的签到日历），
# 不固定的话基线跨天就失效 —— 会在日历那一格上冒出 0.1~0.8% 的差异，看着像渲染坏了。
# 想临时换日期：`MINI_FAKE_NOW=2025-01-01 bash tools/xxx.sh`
export MINI_FAKE_NOW="${MINI_FAKE_NOW:-2024-03-15}"
cd "$(dirname "$0")/.."
OUT="${1:-target/baseline_ia}"
CMP="${2:-}"
BIN=./target/release/mini-app-window
[ -x "$BIN" ] || { echo "先 cargo build --release"; exit 1; }
fail=0

shot() { # 名字, 之后是 mini-app-window 的参数
    local name="$1"; shift
    rm -rf target/mini-storage
    "$BIN" "$@" --snapshot "$OUT/$name" >/dev/null 2>&1 \
        || { echo "  ❌ $name 跑挂了：$BIN $* --snapshot $OUT/$name"; fail=1; return; }
    printf '  ✓ %s\n' "$name"
}

rm -rf "$OUT"
echo "### 交互快照 -> ${OUT} （都等动画落定，避开倒计时页面）"
# 坐标不是猜的：picker 区域用 MINI_PICKER_LOG 量过（内容坐标 y=834，scroll 300 → 视口 534），
# Modal 按钮位置由 host::ui_overlay::modal_layout 的算式得出（居中 280 宽，按钮条高 50，
# 单行内容时 button_y≈360）—— 坐标一旦落空，下面的「空转守卫」会当场报错。
PICKER_TAP="--touch 180,540,50"
MODAL_EVAL="wx.showModal({title:'提示',content:'确认退出登录？',success(){}})"
# picker：点在 selector 上 → 底部面板弹出；等 0.6s 让入场位移动画走完
shot picker_open    sample-app --route pages/showcase/showcase --settle 0 --time 2 --scroll 300 \
    $PICKER_TAP --wait 0.6
# picker：面板弹着滚一格再点「确定」→ 回调页面 + 面板退场
shot picker_confirm sample-app --route pages/showcase/showcase --settle 0 --time 2 --scroll 300 \
    $PICKER_TAP --wait 0.6 --touch 300,380,50 --wait 0.2 --touch 320,300,50 --wait 0.8
# Modal：确定按钮的按压态（按下不抬手）与抬手后关闭
shot modal_press    sample-app --route pages/profile/profile --settle 0 --time 2 \
    --eval "$MODAL_EVAL" --swipe-hold 250,385,250,385,2
shot modal_confirm  sample-app --route pages/profile/profile --settle 0 --time 2 \
    --eval "$MODAL_EVAL" --touch 250,385,30 --wait 0.3
# 按压态：按住列表项不放的那一帧（命中 + set_button_pressed + :active）
shot press_hold     sample-app --route pages/profile/profile --settle 0 --time 2 \
    --swipe-hold 180,300,180,300,2
# 长按：600ms 触发 longpress（触摸状态机 + 帧驱动的 tick）
shot longpress      sample-app --route pages/profile/profile --settle 0 --time 2 --touch 180,300,600
n=$(find "$OUT" -name '*.png' 2>/dev/null | wc -l | tr -d ' ')
echo "共 $n 张"

# ── 空转守卫 ──
# 这套检查最大的风险不是失败，而是**没测到**：坐标落空一像素，面板就没弹出来，
# 快照退化成普通页面，前后自然一致，于是全绿。damage-check 第一版就这么空转过。
echo "### 空转守卫（确认面板/弹窗真的出现过）"
rm -rf target/mini-storage
"$BIN" sample-app --route pages/showcase/showcase --settle 0 --time 2 --scroll 300 \
    --snapshot "$OUT/_plain" >/dev/null 2>&1
guard() { # 名字, 图A, 图B, 最小差异百分比
    local name="$1" a="$2" b="$3" min="$4"
    local pct
    pct=$(python3 tools/pixdiff.py "$a" "$b" 2>/dev/null | tail -1 \
        | sed -n 's/.*(\([0-9.]*\)%).*/\1/p')
    [ -z "$pct" ] && pct=0
    if awk -v p="$pct" -v m="$min" 'BEGIN{exit !(p>=m)}'; then
        printf '  ✓ %-22s 差异 %s%% ≥ %s%%\n' "$name" "$pct" "$min"
    else
        printf '  ✗ %-22s 差异只有 %s%%（应 ≥ %s%%）—— 交互没生效，这条检查是空转\n' \
            "$name" "$pct" "$min"
        fail=1
    fi
}
# 面板占屏幕下半部分，弹出与没弹出至少差 20%
guard picker_面板真的弹出 "$OUT/_plain/pages/showcase/showcase/rust.png" \
    "$OUT/picker_open/pages/showcase/showcase/rust.png" 20
# 按下确定 vs 抬手关闭：一个有弹窗一个没有，至少差 10%
guard modal_按钮真的命中 "$OUT/modal_press/pages/profile/profile/rust.png" \
    "$OUT/modal_confirm/pages/profile/profile/rust.png" 10

echo "### 手势归属断言（时间相关，比日志不比像素）"
expect_log() { # 名字, 期望的正则, 之后是参数
    local name="$1" pat="$2"; shift 2
    rm -rf target/mini-storage
    local log
    log=$(MINI_SCROLL_LOG=1 "$BIN" "$@" 2>&1)
    if grep -qE "$pat" <<<"$log"; then
        printf '  ✓ %-26s %s\n' "$name" "$pat"
    else
        printf '  ✗ %-26s 期望 %s，实际手势日志：\n%s\n' "$name" "$pat" \
            "$(grep -E '🖐|可滚|max_scroll' <<<"$log" | head -6)"
        fail=1
    fi
}
# 首页那排横向卡片上竖着划：不该被横向容器吃掉，应交给页面
expect_log 横滑卡片上竖划走页面 '🖐 手势归属：.*-> Page' \
    sample-app --route pages/index/index --settle 0 --drag 8x12 --snapshot target/_ia_probe
# 纵向长列表拖动：页面滚动真的发生（位置 > 0）
expect_log 长列表能滚起来 '滚动位置 *[1-9]|position=[1-9]|-> Page' \
    sample/news-app --route pages/home/home --settle 0 --drag 10x20 --snapshot target/_ia_probe
rm -rf target/mini-storage target/_ia_probe

if [ -n "$CMP" ]; then
    echo "### 与 ${CMP} 逐像素对比"
    same=0; diff=0; miss=0
    for f in $(find "$OUT" -name '*.png' | sort); do
        rel="${f#"$OUT"/}"
        old="$CMP/$rel"
        [ -f "$old" ] || { printf '  ⚠️  %-44s 基线缺图\n' "$rel"; miss=$((miss+1)); continue; }
        d=$(python3 tools/pixdiff.py "$old" "$f" 2>&1 | tail -1)
        case "$d" in
            *完全一致*) same=$((same+1)) ;;
            *) printf '  ✗ %-44s %s\n' "$rel" "$d"; diff=$((diff+1)) ;;
        esac
    done
    echo "一致 $same / 不一致 $diff / 缺图 $miss"
    { [ "$diff" != 0 ] || [ "$miss" != 0 ]; } && fail=1
fi

[ "$fail" = 0 ] && echo "✅ 交互路径回归通过" || { echo "❌ 交互路径回归失败"; exit 1; }
