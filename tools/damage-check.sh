#!/bin/bash
# 「setData 增量重绘」对照验证：同一次 setData 分别走增量路径与强制整帧路径，
# 截图必须逐字节一致。少画一块的代价是残留脏像素，所以这道校验是增量失效的前提。
# 快照回归统一用**固定日期**：有页面用 `new Date()` 高亮当天（news-app 的签到日历），
# 不固定的话基线跨天就失效。想临时换：`MINI_FAKE_NOW=2025-01-01 bash tools/damage-check.sh`
export MINI_FAKE_NOW="${MINI_FAKE_NOW:-2024-03-15}"

cd "$(dirname "$0")/.."
pkill -9 mini-app-window 2>/dev/null

snap() { # route eval outdir [env]
  local route="$1" ev="$2" out="$3" extra="$4"
  rm -rf "$out"
  if [ -n "$extra" ]; then
    env "$extra" MINI_LAYOUT_LOG=1 ./target/release/mini-app-window sample-app \
      --route "$route" --time 2 --eval "$ev" --snapshot "$out" > "/tmp/$(basename "$out").log" 2>&1 &
  else
    MINI_LAYOUT_LOG=1 ./target/release/mini-app-window sample-app \
      --route "$route" --time 2 --eval "$ev" --snapshot "$out" > "/tmp/$(basename "$out").log" 2>&1 &
  fi
  local P=$!
  sleep 9
  kill -0 $P 2>/dev/null && kill -9 $P
}

FAIL=0
check() { # name route eval
  local name="$1" route="$2" ev="$3"
  snap "$route" "$ev" "target/dm_${name}_on" ""
  snap "$route" "$ev" "target/dm_${name}_off" "MINI_NO_DAMAGE=1"
  echo "--- $name  ($route) ---"
  grep -o 'setData .*' "/tmp/dm_${name}_on.log" | tail -1
  python3 tools/cmp-damage.py "$route" "$name" || FAIL=1
}

check pk    pages/components/components '__callPageMethod("onPickerChange",{"value":2})'
check qty   pages/detail/detail          '__callPageMethod("onQtyPlus",{})'
check slot  pages/showcase/showcase      '__callPageMethod("onSlot",{"value":1})'
check spec  pages/detail/detail          '__callPageMethod("onOpenSpec",{})'
check tab   pages/category/category      '__callPageMethod("onSelectCategory",{"index":2})'

echo
if [ $FAIL -eq 0 ]; then echo "全部一致 ✅"; else echo "存在不一致 ❌"; fi
exit $FAIL
