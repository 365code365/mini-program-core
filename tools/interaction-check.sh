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
# 渲染后端统一用 **CPU**：逐字节基线以 CPU 后端为准（GPU 的抗锯齿与混合随驱动变化，做不了逐字节基线）。
# 无头本来就默认 cpu，这里再显式写一次是双保险：交互窗体默认 auto，哪天无头的默认值被改了，
# 基线会悄悄跑成另一个后端而没人发现。想让同一套脚本测 GPU：`MINI_BACKEND=gpu bash tools/xxx.sh`
export MINI_BACKEND="${MINI_BACKEND:-cpu}"
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
# Modal 按钮位置由 host::ui_overlay::geometry::modal_geom 得出（WeUI 2.x：居中 320 宽，
# 按钮条高 56，单标题单行内容时按钮条 y≈369~425，确定在右半边 x>187）—— 坐标一旦落空，
# 下面的「空转守卫」会当场报错。
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
# slider：showcase 的「音量」滑块（scroll 300 时圆点在视口 171,480；轨道 121.5..203.5，见
# renderer/components/slider.rs 的 slider_track_span）。抓着圆点横拖 24px：60 → 89。
#   · 按住不放的那一帧：滑块必须画出来（按压态样式曾把 slider 借用的颜色/直径字段整份换掉，
#     拖动全程滑块是隐形的，只剩数值在跳）；
#   · 松手：bindchange 回写 volume，标题变成「音量 89」。
SLIDER_DRAG="171,480,195,480,8"
shot slider_hold    sample-app --route pages/showcase/showcase --settle 0 --time 2 --scroll 300 \
    --swipe-hold $SLIDER_DRAG
shot slider_drag    sample-app --route pages/showcase/showcase --settle 0 --time 2 --scroll 300 \
    --swipe $SLIDER_DRAG --wait 0.3
# 右上角胶囊（showcase 是 custom 导航栏）：位置 281,24 87×32（= getMenuButtonBoundingClientRect），
# 左半「•••」弹关于（Modal），见 app_window/capsule_host.rs
shot capsule_about  sample-app --route pages/showcase/showcase --settle 0 --time 2 --touch 300,40 --wait 0.3
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
# 点胶囊左半：关于弹窗 + 遮罩盖住整页（_plain 滚了 300，所以另拍一张不滚的作对照）
rm -rf target/mini-storage
"$BIN" sample-app --route pages/showcase/showcase --settle 0 --time 2 --snapshot "$OUT/_plain_top" >/dev/null 2>&1
guard capsule_关于真的弹出 "$OUT/_plain_top/pages/showcase/showcase/rust.png" \
    "$OUT/capsule_about/pages/showcase/showcase/rust.png" 20
# slider 只比被操作的那一块（`--box`，2x 像素）：这一页有按墙上时钟走的 GIF，整页比较会被它淹没
box_pct() { # 图A, 图B, 矩形 → 差异百分比
    local pct
    pct=$(python3 tools/pixdiff.py "$1" "$2" --box "$3" 2>/dev/null | tail -1 \
        | sed -n 's/.*(\([0-9.]*\)%).*/\1/p')
    echo "${pct:-0}"
}
guard_box() { # 名字, 图A, 图B, 矩形, 最小, 最大
    local name="$1" pct
    pct=$(box_pct "$2" "$3" "$4")
    if awk -v p="$pct" -v lo="$5" -v hi="$6" 'BEGIN{exit !(p>=lo && p<=hi)}'; then
        printf '  ✓ %-22s 区域差异 %s%% ∈ [%s, %s]\n' "$name" "$pct" "$5" "$6"
    else
        printf '  ✗ %-22s 区域差异 %s%%（应在 [%s, %s]）\n' "$name" "$pct" "$5" "$6"
        fail=1
    fi
}
SC=pages/showcase/showcase/rust.png
SLIDER_BOX=230,935,540,1000   # 轨道 + 圆点 + 数值（实测：拖 24px 后差 11.7%）
TITLE_BOX=230,880,540,925     # 标题「音量 N」（实测：60→89 差 2.8%）
# 圆点真的挪了（拖动推进了值）
guard_box slider_拖动推进了值 "$OUT/_plain/$SC" "$OUT/slider_hold/$SC" "$SLIDER_BOX" 5 100
# 按住的那一帧与松手后**逐像素一致**：滑块在拖动中是画出来的（隐形时这里差一整条轨道）
guard_box slider_拖动中画得出来 "$OUT/slider_hold/$SC" "$OUT/slider_drag/$SC" "$SLIDER_BOX" 0 0
# 标题只在松手后变：changing 不改页面，change 回写了 volume
guard_box slider_松手才发change "$OUT/slider_hold/$SC" "$OUT/slider_drag/$SC" "$TITLE_BOX" 1 100

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
# slider 上横拖归滑块、竖划归页面（从前一按到滑块整次触摸就被吃掉，手指落在那一行整页滚不动）
expect_log slider上横拖归滑块 '🖐 手势归属：Undecided -> Slider' \
    sample-app --route pages/showcase/showcase --settle 0 --time 2 --scroll 300 \
    --swipe $SLIDER_DRAG --snapshot target/_ia_probe
expect_log slider上竖划走页面 '🖐 手势归属：Undecided -> Page' \
    sample-app --route pages/showcase/showcase --settle 0 --time 2 --scroll 300 \
    --swipe 171,480,171,300,8 --snapshot target/_ia_probe
# 纵向长列表拖动：页面滚动真的发生（位置 > 0）
expect_log 长列表能滚起来 '滚动位置 *[1-9]|position=[1-9]|-> Page' \
    sample/news-app --route pages/home/home --settle 0 --drag 10x20 --snapshot target/_ia_probe
# 胶囊右半「⊙」= 返回宿主（无头模式只记日志；交互窗体关窗口）
expect_log 胶囊右半是返回宿主 '胶囊 close：返回宿主' \
    sample-app --route pages/showcase/showcase --settle 0 --time 2 --touch 350,40 --snapshot target/_ia_probe
# 按在胶囊上滑出去再松手：什么也不发（与原生按钮一致）。先做对照：同样是 --swipe，
# 只在胶囊里挪一点再松手要触发 —— 不然下面那条「没触发」可能只是根本没按到胶囊
expect_log 胶囊里挪一点照样算点 '胶囊 menu：查看关于' \
    sample-app --route pages/showcase/showcase --settle 0 --time 2 --swipe 300,40,304,42,3 --snapshot target/_ia_probe
if MINI_SCROLL_LOG=1 "$BIN" sample-app --route pages/showcase/showcase --settle 0 --time 2 \
    --swipe 300,40,200,200,6 --snapshot target/_ia_probe 2>&1 | grep -q '胶囊 '; then
    printf '  ✗ %-26s 滑出胶囊松手不该触发\n' 胶囊滑出去不算点; fail=1
else
    printf '  ✓ %-26s 没有触发\n' 胶囊滑出去不算点
fi
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
