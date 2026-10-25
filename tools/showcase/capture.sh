#!/usr/bin/env bash
# 真机 / 模拟器上逐页打开示例小程序截图（展示视频素材，落在 target/_video/shots）。行格式：目录名 路由 query；SERIAL 选设备（默认 emulator-5554）
set -u
D(){ adb -s "${SERIAL:-emulator-5554}" "$@" </dev/null; }
cd "$(dirname "$0")/../.."
U=$(sed -n 5p sample/测试服务器.txt | python3 -c "import sys,urllib.parse as u;s=u.urlsplit(sys.stdin.read().strip());print(f'{s.scheme}://{s.netloc}')")
mkdir -p target/_video/shots
while read -r app path query; do
  [ -z "$app" ] && continue
  [ -s "target/_video/shots/${app}__$(echo "$path" | tr '/' '_').png" ] && continue
  ap=$(python3 -c "import json;print(json.load(open('sample/$app/project.config.json'))['appid'])")
  q=${query:-}; [ "$q" = "-" ] && q=""
  L=$(python3 -c "import json,sys;import urllib.parse as u;print(json.dumps({'key':sys.argv[1],'path':sys.argv[2],'query':dict(u.parse_qsl(sys.argv[3])),'scene':1001},separators=(',',':')))" "$ap" "$path" "$q")
  D shell "am start -S -n dev.minirender.example/.MainActivity --es storeLink '$L' --es storeUrl $U" >/dev/null 2>&1
  /bin/sleep 7
  name="${app}__$(echo "$path" | tr '/' '_')"
  if D shell dumpsys window | grep -m1 mCurrentFocus | grep -q "dev.minirender.example/"; then
    D exec-out screencap -p > "target/_video/shots/$name.png"; echo "ok $name"
  else
    echo "SKIP ${name}（前台不是示例 App）"
  fi
done <<'EOF'
music pages/discover/discover -
music pages/playlist/playlist id=p2
music pages/library/library -
train pages/index/index -
train pages/list/list -
train pages/book/book -
recipe pages/index/index -
recipe pages/detail/detail id=2
recipe pages/search/search -
notes pages/index/index -
notes pages/edit/edit id=3
notes pages/tags/tags -
kanban pages/board/board -
kanban pages/card/card id=1
kanban pages/stats/stats -
stocks pages/watch/watch -
stocks pages/detail/detail code=600519
stocks pages/trade/trade code=601318&side=buy
reader pages/shelf/shelf -
reader pages/store/store -
reader pages/reader/reader id=b1
smart-home pages/home/home -
smart-home pages/ac/ac id=ac1
smart-home pages/energy/energy -
game-2048 pages/game/game seed=7
game-2048 pages/rank/rank -
game-2048 pages/rules/rules -
calendar pages/month/month -
calendar pages/edit/edit id=3
calendar pages/list/list -
parking pages/index/index -
parking pages/lots/lots -
parking pages/pay/pay -
marathon pages/index/index -
marathon pages/signup/signup group=1
marathon pages/mine/mine -
chat pages/chat/chat id=c19
chat pages/sessions/sessions -
EOF
echo DONE
