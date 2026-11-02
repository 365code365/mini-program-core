#!/usr/bin/env bash
# iPhone 真机验宿主原生组件（Swift 原生示例 dev.minirender.example.native，先用 `IOS_APP=native bash tools/ios-debug.sh` 装好）：
# 内置示例的地图 / web-view / 直播三页带 `?auto=1&chain=1` 串着自测，每页一行 `NC-CHECK …`，判据同 tools/android-native-components.sh。
#
#   bash tools/ios-native-components.sh                 # 第一台连着的 iPhone
#   IOS_UDID=<udid> bash tools/ios-native-components.sh
#
# 手势（拖地图、捏合）devicectl 送不了触摸，这里不测：引擎侧由 tests/same_layer_view_e2e.rs 守着，Android 脚本真机拖一下
set -u
cd "$(dirname "$0")/.."
OUT=target/_native_components
mkdir -p "$OUT"
LOG="$OUT/ios.log"
U=${IOS_UDID:-$(xcrun devicectl list devices 2>/dev/null | awk '/physical/ && /connected/ {for (i=1;i<=NF;i++) if ($i ~ /^[0-9A-F]{8}-[0-9A-F]{16}$/) print $i}' | head -1)}
[ -n "$U" ] || { echo "❌ 没找到连着的 iPhone（IOS_UDID=…）"; exit 1; }
# devicectl 自己的参数与 App 的启动参数用 `--` 隔开（不隔的话 -route 被当成 devicectl 的选项）
xcrun devicectl device process launch --device "$U" --console --terminate-existing dev.minirender.example.native \
    -- -autoOpen 1 -route "pages/map/map?auto=1&chain=1" > "${LOG}" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null' EXIT
end=$(( $(date +%s) + 120 ))
until grep -q "NC-CHECK done" "${LOG}"; do
    [ "$(date +%s)" -ge "$end" ] && { echo "❌ 120s 内没跑完（看 ${LOG}）"; break; }
    sleep 0.5
done
fail=0
for k in map webview live; do
    line=$(grep -oE "NC-CHECK $k \{[^}]*(\}[^\"]*)?" "${LOG}" | tail -1 | sed 's/\\"/"/g')
    if [ -z "$line" ]; then echo "❌ $k：没有结果"; fail=1
    elif echo "$line" | grep -q '"ok":true'; then echo "✅ $line"
    else echo "❌ $line"; fail=1; fi
done
exit $fail
