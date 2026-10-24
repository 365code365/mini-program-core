#!/usr/bin/env bash
# 把一个小程序目录打包上传到 mini-store（同 ID 再传 = 升级，版本号 +1）。
#
#   MINI_STORE_PASS=… bash tools/mini-store/upload.sh <小程序目录> <id> [名称] [描述]
#
# AppID 取包里 project.config.json 的 appid；没有就沿用这个 id 上次的，再没有就由服务器分配
# （服务器会把最终的 AppID 写回包里的 project.config.json）。要强制指定：MINI_STORE_APPID=wx…
#
# 环境变量：MINI_STORE_URL（默认 http://64.83.22.217:9630）、MINI_STORE_USER（默认 admin）、MINI_STORE_PASS（必填）
set -euo pipefail
DIR="${1:?用法：upload.sh <小程序目录> <id> [名称] [描述]}"
ID="${2:?缺 id}"
NAME="${3:-}"
DESC="${4:-}"
URL="${MINI_STORE_URL:-http://64.83.22.217:9630}"
USER_="${MINI_STORE_USER:-admin}"
: "${MINI_STORE_PASS:?要 MINI_STORE_PASS}"
[ -f "$DIR/app.json" ] || { echo "✗ $DIR 下没有 app.json，不是小程序目录" >&2; exit 1; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
(cd "$DIR" && zip -qr -X "$TMP/pkg.zip" . -x '*.DS_Store' -x '__MACOSX/*' -x '*/node_modules/*' -x 'node_modules/*')

# 登录拿 token（密码走 stdin 进 python，不出现在进程参数里）
MS_USER="$USER_" python3 -c '
import json, os
print(json.dumps({"username": os.environ["MS_USER"], "password": os.environ["MINI_STORE_PASS"]}))' \
  | curl -sS -H 'Content-Type: application/json' --data-binary @- "$URL/api/login" > "$TMP/login.json"
TOKEN=$(python3 -c '
import json, sys
r = json.load(open(sys.argv[1]))
if "token" not in r: sys.exit("✗ 登录失败：" + r.get("error", "?"))
print(r["token"])' "$TMP/login.json")

Q=$(MS_NAME="$NAME" MS_DESC="$DESC" python3 -c '
import os, urllib.parse
q = {"name": os.environ["MS_NAME"]}
if os.environ["MS_DESC"]: q["description"] = os.environ["MS_DESC"]
if os.environ.get("MINI_STORE_APPID"): q["appid"] = os.environ["MINI_STORE_APPID"]
print(urllib.parse.urlencode(q))')

curl -sS -X PUT -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/zip' \
  --data-binary "@$TMP/pkg.zip" "$URL/api/admin/apps/$ID?$Q" > "$TMP/put.json"
python3 -c '
import json, sys
m = json.load(open(sys.argv[1]))
if "error" in m: sys.exit("✗ 上传失败：" + m["error"])
print("✓ %s「%s」%s v%d  %d 个文件  %d 页  %.2f MB" % (m["id"], m["name"], m.get("appId", ""), m["version"], m["files"], m["pages"], m["size"] / 1048576))' "$TMP/put.json"
