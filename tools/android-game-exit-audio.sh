#!/usr/bin/env bash
# 小游戏退出 / 切后台之后不再出声（真机速测，Flutter 示例或原生示例）：一个游戏十秒上下出结论。
#
#   bash tools/android-game-exit-audio.sh [AppID …]             # 不给 = 商店列表里全部小游戏
#   MINI_APP=native bash tools/android-game-exit-audio.sh …     # 原生示例（GameActivity）
#   ANDROID_SERIAL=emulator-5554 bash tools/android-game-exit-audio.sh   # 模拟器（默认一加 PLY110）
#
# 看系统音频服务里本进程的 AAudio 播放器（`dumpsys audio`）：它在 = 有出声的通路。每个游戏：
#   ① 进游戏（等到「🎮 第一帧上屏」）：state:started；
#   ② Home 进后台：paused；回前台：started；
#   ③ 退出（返回键与胶囊 ⊙ 轮着用）：8 秒内消失，且有「🎮 关掉实例」。正常 0.2~0.6s（原生示例 1s 上下）；
#      引擎线程正忙时（Cocos 加载中一帧 JS 跑几百毫秒）销毁排在那一帧之后，模拟器上见过 2~3s。
# 「首帧」那一列含下载：本机没装过的游戏先从商店下（farm001 12MB，模拟器上 28s）。
# 2026-11 踩过：退出后播放器一直 started、背景音乐接着响 —— 字节码缓存漏一份 JS 上下文引用，
# GameAudio 一直不析构（src/js/bytecode_cache.rs 的 run，doc/踩坑记录.md）。
#
# 等待一律按墙上时钟轮询（首帧、播放器状态），不写死 sleep。日志开一条 logcat 流写进本地文件再 grep：
# 一加 PLY110 的 logcat 环只有 256KB，ColorOS 几十秒就冲掉，每次 `logcat -d` 从头翻既慢又会漏（踩过：
# 一次轮询 0.6s，「等 40 秒」实际等了 122 秒，要找的那行早被冲掉了）。
# 手机是几个会话共用的：每步先确认前台是示例 App，不是就停（steering 30）。
# 本机装过的版本就能测：手机连不上商店时示例 App 打开本地那版；本地也没有的记 SKIP。
set -u
cd "$(dirname "$0")/.."
SERIAL="${ANDROID_SERIAL:-3B669L00NA000000}"
A="$HOME/Library/Android/sdk/platform-tools/adb"
D() { "$A" -s "$SERIAL" "$@" </dev/null; }
if [ "${MINI_APP:-flutter}" = native ]; then PKG=dev.minirender.example; else PKG=dev.minirender.mini_render_example; fi
STORE="${MINI_STORE:-https://mini-program.51aihelp.com}"
GAMES=("$@")
if [ ${#GAMES[@]} -eq 0 ]; then
  GAMES=($(curl -s --max-time 8 "$STORE/api/apps?kind=all" | python3 -c '
import json, sys
for a in json.load(sys.stdin)["apps"]:
    if a.get("kind") == "game": print(a.get("appId") or a["id"])'))
  [ ${#GAMES[@]} -gt 0 ] || { echo "⚠ 拉不到商店列表，请直接给 AppID"; exit 1; }
fi
[ "$(D get-state 2>/dev/null)" = device ] || { echo "⚠ 设备 ${SERIAL} 不在线（adb devices 看不到或没授权）"; exit 1; }
# 从干净的进程开始：上一次（或手动）留在后台的游戏实例，深链再开同一个游戏只会把它拉回前台、不出新的首帧
D shell am force-stop "$PKG"

OUT=target/_game_exit_audio; mkdir -p "$OUT"
LOG="$OUT/logcat.txt"; : > "$LOG"
# 只要这几个 tag；-T 1 = 从现在起（不翻旧日志）。直接起 adb、不经 D()：经函数放到后台的是一个子 shell，
# $! 是它的号，退出时 kill 掉的是子 shell，adb logcat 留成孤儿一直跑（踩过：跑了 7 次留下 7 个）
"$A" -s "$SERIAL" logcat -v brief -T 1 -s 'mini-game:I' 'flutter:I' 'mini-render:I' 'mini-example:I' </dev/null >> "$LOG" 2>/dev/null &
LC=$!
trap 'kill $LC 2>/dev/null' EXIT

ms() { python3 -c 'import time; print(int(time.time() * 1000))'; }
sec() { awk "BEGIN { printf \"%.1fs\", $1 / 1000 }"; }
# 轮询：until_ <秒> <命令…>，按墙上时钟算（每次检查本身要几百毫秒，按次数算会拖成几倍）
until_() { local end=$(( $(ms) + $1 * 1000 )); shift; while :; do "$@" && return 0; [ "$(ms)" -lt "$end" ] || return 1; sleep 0.2; done; }
# 本进程 AAudio 播放器的状态，如 "started" / "paused"；多个用逗号连；没有为空
aa() { D shell dumpsys audio | grep -E "type:AAudio u/pid:[0-9]+/$1 " | grep -oE "state:[a-z]+" | cut -d: -f2 | paste -sd, -; }
is_state() { [ "$(aa "$PID")" = "$1" ]; }
no_player() { [ -z "$(aa "$PID")" ]; }
fg() { D shell dumpsys window | grep -m1 mCurrentFocus | grep -q "$PKG"; }
not_fg() { ! fg; }
# 本局（自 L0 行起）的日志
log_since() { tail -n "+$((L0 + 1))" "$LOG"; }
first_frame() { log_since | grep -q "第一帧上屏"; }
cant_open() { log_since | grep -qE "打不开|没有这个小程序"; }
# 游戏脚本报的第一个错：game.js 顶层抛错（kind script），或入口模块就注册失败（kind module，语法不支持之类）
script_error() { log_since | grep -m1 -E '"kind":"(script|module)"' | sed -nE 's/.*"message":"([^"]*)".*/\1/p'; }
# 首帧到了 / 打不开 / 脚本报错之后又等了 8 秒还没首帧（包本身跑不起来，不用干等 40 秒）
settled() { first_frame || cant_open || { [ -n "$(script_error)" ] && [ $(( $(ms) - t0 )) -gt 8000 ]; }; }
closed_log() { log_since | grep -q "🎮 关掉实例"; }

pass=0; fail=0; skip=0; k=0
printf '%-22s %-8s %-9s %-9s %-9s %-10s %s\n' "游戏" "首帧" "进游戏" "后台" "回前台" "退出方式" "退出后（播放器消失用时）"
for G in "${GAMES[@]}"; do
  k=$((k + 1))
  sleep 0.3 # 让上一局的尾巴落进日志文件，L0 才不会把它算进这一局
  L0=$(wc -l < "$LOG" | tr -d ' ')
  t0=$(ms)
  D shell "am start -a android.intent.action.VIEW -d 'minirender://open?appid=$G' $PKG" >/dev/null 2>&1
  until_ 40 settled
  if ! first_frame; then
    if cant_open; then printf '%-22s SKIP（本机没装过、也连不上商店）\n' "$G"; skip=$((skip + 1))
    else printf '%-22s FAIL %s 没等到首帧%s\n' "$G" "$(sec $(( $(ms) - t0 )))" "$(e=$(script_error); [ -n "$e" ] && echo "（game.js 报错：${e}）")"; fail=$((fail + 1)); fi
    D shell input keyevent 4
    until_ 5 fg
    continue
  fi
  t1=$(ms)
  PID=$(D shell pidof "$PKG" | tr -d '\r')
  fg || { echo "⚠ 前台不是 ${PKG}（手机被别的会话占着？）"; exit 1; }
  until_ 3 is_state started; s_in=$(aa "$PID")
  if [ -z "$s_in" ]; then printf '%-22s SKIP（没开音频设备：离线静音模式）\n' "$G"; skip=$((skip + 1)); D shell input keyevent 4; until_ 3 fg; continue; fi
  # Home：刚开完游戏时 ColorOS 还在播启动动画，这一下可能被吃掉（App 留在前台，量出来的「后台」其实是前台）。
  # 先确认真的退到了桌面，没退就再按一次
  D shell input keyevent 3
  until_ 2 not_fg || { D shell input keyevent 3; until_ 3 not_fg; }
  # 引擎线程正卡在一帧长 JS 里（farm001 冷启动加载时一帧 1~4.5s）时，暂停排在那一帧之后；那时还没开播（voices 0、
  # 峰值 0），听不出来。等到 8 秒，超过 1 秒的把用时标出来
  tb=$(ms); until_ 8 is_state paused; s_bg=$(aa "$PID"); tb=$(( $(ms) - tb ))
  [ "$s_bg" = paused ] && [ "$tb" -gt 1000 ] && s_bg="paused($(sec $tb))"
  D shell monkey -p "$PKG" -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1
  until_ 5 fg || { echo "⚠ 回前台之后前台不是 ${PKG}：$(D shell dumpsys window | grep -m1 mCurrentFocus)"; exit 1; }
  until_ 3 is_state started; s_fg=$(aa "$PID")
  # 退出：单数个用返回键，双数个点胶囊 ⊙（引擎报的胶囊矩形右半，逻辑像素 × dpr；没有胶囊就退回返回键）。
  # 点之前等胶囊停下来：横屏游戏回前台要先转回横屏、胶囊再挪几次（踩过：转完之前点，坐标落在竖屏的屏外，点空了）
  how=返回键
  if [ $((k % 2)) = 0 ]; then
    n0=-1; for _ in 1 2 3 4 5 6; do n1=$(log_since | grep -c "🎮 胶囊"); [ "$n1" = "$n0" ] && break; n0=$n1; sleep 0.7; done
  fi
  cap=$(log_since | grep "🎮 胶囊" | tail -1 | sed -nE 's/.*胶囊 ([0-9.]+), ([0-9.]+), ([0-9.]+), ([0-9.]+).*/\1 \2 \3 \4/p')
  dpr=$(log_since | grep -m1 "🎮 建实例" | sed -nE 's/.*@([0-9.]+)[，,].*/\1/p')
  if [ $((k % 2)) = 0 ] && [ -n "$cap" ] && [ -n "$dpr" ]; then
    read -r tx ty <<<"$(python3 -c "x,y,w,h=map(float,'$cap'.split()); d=float('$dpr'); print(round((x+w*0.75)*d), round((y+h/2)*d))")"
    how="胶囊⊙"
    t2=$(ms); D shell input tap "$tx" "$ty"
  else
    t2=$(ms); D shell input keyevent 4
  fi
  # 引擎线程正忙（加载中一帧 JS 跑几百毫秒）时销毁排在这一帧之后：多给几秒，用时照实打出来
  if until_ 8 no_player; then gone="$(( $(ms) - t2 ))ms"; else gone="还在：$(aa "$PID")"; fi
  # 「关掉实例」在 JS 运行时拆完之后才打（大游戏比关音频晚几百毫秒）
  until_ 5 closed_log
  ok=1
  [ "$s_in" = started ] && [ "${s_bg%%(*}" = paused ] && [ "$s_fg" = started ] && no_player && closed_log || ok=0
  printf '%-22s %-8s %-9s %-9s %-9s %-10s %s%s\n' "$G" "$(sec $((t1 - t0)))" "$s_in" "$s_bg" "$s_fg" "$how" "$gone" "$(closed_log || echo '（没有「关掉实例」）')"
  if [ $ok = 1 ]; then pass=$((pass + 1)); else fail=$((fail + 1)); fi
  # 退出后回到哪：Flutter 示例回商店首页；原生示例的 GameActivity 是深链在新任务里开的，关掉它回到启动它的地方（桌面），
  # 两种都对。下一局的深链会把 App 再拉起来
  until_ 3 fg || true
done
echo "结论：通过 ${pass}，失败 ${fail}，跳过 ${skip}（${PKG}）"
[ $fail = 0 ]
