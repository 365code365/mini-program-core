# tools/android-native-check.sh 的交互那一组（由它 source，用它的 launch / tap / shot / wait_log / changed …）。
#
# 坐标是逻辑像素，从一加 PLY110（423×923，引擎画导航栏）的截图上量的。判据都是可判定的：
# 要么等引擎发出的事件（`[事件] page <路由> depth=N`、`inputFocus`），要么比两张截图某个矩形里
# 变了多少像素 —— 「动了」要 >2%，「回到原样」要 <0.5%。

# 某个区域在两张截图之间变化超过 / 不超过阈值
moved() {   # moved <名字> <截图a> <截图b> <x0,y0,x1,y1> [阈值%]
    local r; r=$(changed "$2" "$3" "$4")
    python3 -c "import sys; sys.exit(0 if $r > ${5:-2} else 1)" && ok "$1（变了 ${r}%）" || bad "$1（只变了 ${r}%）"
}
same() {    # same <名字> <截图a> <截图b> <x0,y0,x1,y1>
    local r; r=$(changed "$2" "$3" "$4")
    python3 -c "import sys; sys.exit(0 if $r < 0.5 else 1)" && ok "$1（差 ${r}%）" || bad "$1（差 ${r}%）"
}
# 等某条 page 事件（自 mark 起）
want_page() {   # want_page <名字> <路由> <depth>
    wait_log "page $2 depth=$3 " 8 >/dev/null && ok "$1" || bad "$1：没等到 page $2 depth=$3"
}
focus_is_dialog() { D shell dumpsys window </dev/null | grep -m1 mCurrentFocus | grep "$PKG" | grep -vq MainActivity; }

check_func() {
    local START
    mark; START=$T0
    echo "【交互】tabBar"
    launch "" || return
    local x
    for x in "159 pages/components/components" "264 pages/apis/apis" "370 pages/host/host" "53 pages/index/index"; do
        set -- $x
        mark; tap "$1" 880
        want_page "点 tab → $2" "$2" 1
    done

    echo "【交互】滚动 / Toast / ActionSheet / Modal（首页）"
    shot f_index0
    swipe 210 700 210 300 400; sleep 1.2; shot f_index1
    moved "上滑 400dp：页面跟着动" f_index0 f_index1 0,150,423,850
    swipe 210 300 210 800 300; sleep 1.5; shot f_index2
    same "滑回顶部：与滑之前一致" f_index0 f_index2 0,150,423,850
    tap 56 522; sleep 0.4; shot f_toast
    moved "Toast 弹出" f_index2 f_toast 100,350,323,600
    sleep 2.2; shot f_toast_gone
    same "Toast 1.5s 后自己消失" f_index2 f_toast_gone 0,150,423,850
    tap 152 522; sleep 0.8; shot f_sheet
    moved "ActionSheet 从底部弹出" f_index2 f_sheet 0,600,423,900
    tap 210 200; sleep 0.8; shot f_sheet_gone
    same "点遮罩收起 ActionSheet" f_index2 f_sheet_gone 0,150,423,900
    tap 248 522; sleep 0.8; shot f_modal
    moved "Modal 弹出" f_index2 f_modal 40,300,383,600

    echo "【交互】navigateTo + 返回键 + 导航栏返回箭头（路由页）"
    launch pages/route/route || return
    mark; tap 88 392
    want_page "navigateTo 详情" pages/detail/detail 2
    sleep 0.6; mark; D shell input keyevent KEYCODE_BACK
    want_page "返回键 → 回到路由页" pages/route/route 1
    sleep 0.6; mark; tap 88 392
    want_page "再进详情" pages/detail/detail 2
    # 引擎画的导航栏：状态栏下面 44dp 高，返回箭头在左边
    sleep 0.8; local sb; sb=$(python3 -c "print($(g statusBar)/$DPR)")
    mark; tap 22 "$(python3 -c "print($sb + 22)")"
    want_page "点导航栏返回箭头 → 回到路由页" pages/route/route 1
    sleep 0.6; mark; tap 287 472
    want_page "switchTab 到「接口」" pages/apis/apis 1
    sleep 0.6; D shell input keyevent KEYCODE_BACK; sleep 1.2
    front && bad "页面栈只剩一页时按返回键，应退出 Activity" || ok "页面栈只剩一页时按返回键：退出 Activity"

    echo "【交互】前后台（路由页的生命周期流水）"
    launch pages/route/route || return
    shot f_life0
    D shell input keyevent KEYCODE_HOME; sleep 1.5
    D shell am start -n "$PKG/.MainActivity" >/dev/null; sleep 2
    front && ok "回到前台" || bad "没回到前台"
    shot f_life1
    moved "回前台后生命周期流水多了 onHide / onShow（且画面在出帧）" f_life0 f_life1 20,660,403,760 1

    echo "【交互】静止页面上的定时器 / 自动播放照常出帧"
    launch pages/perf/perf || return
    shot f_perf0; sleep 2.2; shot f_perf1
    moved "综合场景「已运行 N 秒」每秒 setData" f_perf0 f_perf1 10,255,200,290 1
    launch pages/swiper/swiper || return
    shot f_sw0; sleep 3.5; shot f_sw1
    moved "轮播自动播放" f_sw0 f_sw1 25,155,398,320

    echo "【交互】storage 落盘（冷启动两次，「本页累计打开次数」要变）"
    launch pages/storage/storage || return
    shot f_st0
    launch pages/storage/storage || return
    shot f_st1
    moved "冷启动后计数从 storage 读回并 +1" f_st0 f_st1 30,150,100,205 1

    echo "【交互】网络（wx.request → httpbin.org）"
    launch pages/network/network || return
    shot f_net0; tap 77 216; sleep 5; shot f_net1
    moved "GET + query 回包显示出来" f_net0 f_net1 20,285,403,340 1

    echo "【交互】picker（选择器页）"
    launch pages/picker/picker || return
    shot f_pk0; tap 200 173; sleep 1; shot f_pk1
    moved "点「城市」弹出 picker 面板" f_pk0 f_pk1 0,600,423,900
    tap 210 150; sleep 1; shot f_pk2
    same "点遮罩收起面板" f_pk0 f_pk2 0,150,423,900

    echo "【交互】custom 导航栏页的胶囊"
    launch pages/navcustom/navcustom || return
    tap 350 "$(python3 -c "print($(g statusBar)/$DPR + 22)")"; sleep 1
    focus_is_dialog && ok "胶囊「•••」→ 弹出关于对话框" || bad "胶囊「•••」没弹出对话框"
    D shell input keyevent KEYCODE_BACK; sleep 0.8
    tap 394 "$(python3 -c "print($(g statusBar)/$DPR + 22)")"; sleep 1.2
    front && bad "胶囊「⊙」应结束 Activity" || ok "胶囊「⊙」→ 结束 Activity"

    echo "【交互】输入框（表单页）"
    launch pages/form/form || return
    mark; tap 250 533
    wait_log "inputFocus" 4 >/dev/null && ok "点输入框：引擎发出 inputFocus" || bad "点输入框没有 inputFocus 事件"
    sleep 1
    if D shell dumpsys input_method </dev/null | grep -q "mInputShown=true"; then ok "软键盘弹出"
    else echo "  ⚠ 软键盘没弹出：MiniProgramView 没接输入法（inputFocus 之后要宿主自己弹键盘、把文字交给 sendText）"; fi

    echo "【交互】转屏：视图改尺寸，引擎跟着重排"
    launch "" || return
    local acc rot
    acc=$(D shell settings get system accelerometer_rotation | tr -d '\r')
    rot=$(D shell settings get system user_rotation | tr -d '\r')
    D shell settings put system accelerometer_rotation 0
    D shell settings put system user_rotation 1; sleep 2.5
    shot f_land
    D shell settings put system user_rotation "${rot:-0}"
    D shell settings put system accelerometer_rotation "${acc:-1}"; sleep 1.5
    # 横屏时画面右半边（逻辑 x>423）要有内容：引擎没跟着改尺寸的话，那一大块是竖屏画面最右一列拉过去的，每行一个颜色
    local rows
    rows=$(python3 - "$OUT/f_land.png" "$DPR" <<'EOF'
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert('RGB'); d = float(sys.argv[2])
w, h = im.size
if w < h: print(-1); sys.exit()
x0, x1 = round(500 * d), w - round(80 * d)
n = 0
for y in range(round(100 * d), h - round(60 * d), 4):
    if len(set(im.getpixel((x, y)) for x in range(x0, x1, 3))) > 1: n += 1
print(n)
EOF
)
    [ "$rows" = -1 ] && bad "没转成横屏（截图还是竖的）" || {
        [ "$rows" -gt 20 ] && ok "横屏右半边有内容（${rows} 行不是纯色）" || bad "横屏右半边是空的 / 拉伸的（只有 ${rows} 行有变化）"; }
    D shell pidof "$PKG" >/dev/null && ok "转屏前后进程活着" || bad "转屏后进程没了"
    T0=$START
    local c; c=$(crashed)
    [ -z "$c" ] && ok "整组交互没有崩溃（crash 缓冲区）" || bad "交互过程中有崩溃：${c}"
}
