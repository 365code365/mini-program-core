#!/usr/bin/env bash
# 拉 App 上传到 mini-store 的诊断日志（App 里点「上传诊断日志」，会提示一个诊断 id）。
#
#   bash tools/mini-store/diag.sh list                 # 最近的诊断（最新在前）
#   bash tools/mini-store/diag.sh get <id|latest>      # 下载到 target/diag/<id>.json，打印概要与日志里的告警行
#   bash tools/mini-store/diag.sh log <id|latest>      # 只打印日志正文（可接 grep）
#   bash tools/mini-store/diag.sh rm <id>
#
# 环境变量：MINI_STORE_URL（默认 http://64.83.22.217:9630）、MINI_STORE_USER（默认 admin）、
# MINI_STORE_PASS（不给就读 sample/测试服务器.txt 第 7 行 —— 那个文件在 .gitignore 里，不进仓库）
set -euo pipefail
cd "$(dirname "$0")/../.."
URL="${MINI_STORE_URL:-http://64.83.22.217:9630}"
USER_="${MINI_STORE_USER:-admin}"
if [ -z "${MINI_STORE_PASS:-}" ] && [ -f sample/测试服务器.txt ]; then
  MINI_STORE_PASS="$(sed -n 7p sample/测试服务器.txt)"
fi
: "${MINI_STORE_PASS:?要 MINI_STORE_PASS}"
export MINI_STORE_PASS
CMD="${1:-list}"
ID="${2:-latest}"

TOKEN=$(MS_USER="$USER_" python3 -c '
import json, os
print(json.dumps({"username": os.environ["MS_USER"], "password": os.environ["MINI_STORE_PASS"]}))' \
  | curl -sS -H 'Content-Type: application/json' --data-binary @- "$URL/api/login" \
  | python3 -c 'import json,sys; r=json.load(sys.stdin); print(r.get("token") or sys.exit("✗ 登录失败：" + r.get("error","?")))')
api() { curl -sS -H "Authorization: Bearer $TOKEN" "$@"; }

case "$CMD" in
  list)
    api "$URL/api/admin/diag" | python3 -c '
import json, sys, time
for d in json.load(sys.stdin)["items"]:
    t = time.strftime("%m-%d %H:%M:%S", time.localtime((d.get("receivedAt") or 0) / 1000))
    app = d.get("app") or {}
    print("%s  %s  %-8s %-26s %-12s %6d 行  %s" % (d["id"], t, d.get("platform") or "", (d.get("device") or "")[:26],
          app.get("id") or "-", d.get("logLines") or 0, d.get("note") or ""))'
    ;;
  get)
    mkdir -p target/diag
    api "$URL/api/admin/diag/$ID" > target/diag/_tmp.json
    python3 - target/diag/_tmp.json <<'EOF'
import json, os, re, sys
p = sys.argv[1]
d = json.load(open(p))
if "error" in d:
    sys.exit("✗ " + d["error"])
out = os.path.join(os.path.dirname(p), d["id"] + ".json")
os.replace(p, out)
logs = d.get("logs") or ""
print("诊断", d["id"], "→", out)
for k in ("host", "platform", "device", "system", "backend", "app", "store", "note"):
    if d.get(k) not in (None, "", {}):
        print("  %-9s %s" % (k, json.dumps(d[k], ensure_ascii=False) if isinstance(d[k], (dict, list)) else d[k]))
lines = logs.splitlines()
print("  日志 %d 行" % len(lines))
# 引擎的逐帧日志（🩹，debug.mini.damage_log 开着时每帧一行）只算慢帧：> 16ms 的才列出来
def slow(l):
    m = re.search(r"〕\s*([0-9.]+)ms", l)
    return m is not None and float(m.group(1)) > 16
frames = [l for l in lines if "🩹" in l]
slow_frames = [l for l in frames if slow(l)]
if frames:
    print("  引擎帧日志 %d 帧，>16ms 的 %d 帧" % (len(frames), len(slow_frames)))
pat = re.compile(r"(?i) [EF] |\b[EF]/|error|exception|fatal|panic|crash|打不开|失败|⚠|ANR")
bad = [l for l in lines if ("🩹" not in l and pat.search(l)) or slow(l)]
print("  可疑行 %d（最后 40 条）：" % len(bad))
for l in bad[-40:]:
    print("    " + l[:220])
EOF
    ;;
  log)
    api "$URL/api/admin/diag/$ID" | python3 -c 'import json,sys; d=json.load(sys.stdin); sys.stdout.write(d.get("logs") or d.get("error","") + "\n")'
    ;;
  rm)
    api -X DELETE "$URL/api/admin/diag/$ID"; echo
    ;;
  *)
    sed -n 2,10p "$0"; exit 2 ;;
esac
