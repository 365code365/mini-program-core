#!/usr/bin/env bash
# 部署 / 升级 mini-store 到服务器（Debian + systemd + ufw）。
#
#   bash tools/mini-store/deploy.sh                       # 只更新代码并重启（数据、账户不动）
#   MINI_STORE_PASS=… bash tools/mini-store/deploy.sh     # 同时（重新）设置管理账户的密码
#
# MINI_STORE_HOST（默认 root@64.83.22.217）、MINI_STORE_USER（默认 admin）。
# ssh 要密码时先 `export SSHPASS=…`，脚本会自动走 sshpass。
set -euo pipefail
cd "$(dirname "$0")"
HOST="${MINI_STORE_HOST:-root@64.83.22.217}"
USER_="${MINI_STORE_USER:-admin}"
if [ -n "${SSHPASS:-}" ]; then S=(sshpass -e); else S=(); fi
R() { "${S[@]}" ssh -o ConnectTimeout=15 "$HOST" "$@"; }

# --no-xattrs / COPYFILE_DISABLE：别把 macOS 的扩展属性打进去（GNU tar 解的时候每个文件警告一行）
COPYFILE_DISABLE=1 tar --no-xattrs -czf - server.py store_pkg.py landing.py diag.py static mini-store.service | R 'set -e
id mini-store >/dev/null 2>&1 || useradd --system --home /opt/mini-store --shell /usr/sbin/nologin mini-store
mkdir -p /opt/mini-store/app /opt/mini-store/data/apps
tar xzf - -C /opt/mini-store/app
mv /opt/mini-store/app/mini-store.service /etc/systemd/system/mini-store.service
chown -R root:root /opt/mini-store/app
chown -R mini-store:mini-store /opt/mini-store/data
chmod 700 /opt/mini-store/data'

if [ -n "${MINI_STORE_PASS:-}" ]; then
  printf '%s\n' "$MINI_STORE_PASS" | R "sudo -u mini-store env MINI_STORE_DATA=/opt/mini-store/data python3 /opt/mini-store/app/server.py set-password '$USER_'"
fi

R 'set -e
systemctl daemon-reload
systemctl enable mini-store >/dev/null 2>&1
systemctl restart mini-store
command -v ufw >/dev/null && ufw allow 9630/tcp >/dev/null
sleep 1
systemctl is-active mini-store
curl -sS -o /dev/null -w "本机 /api/apps → %{http_code}\n" http://127.0.0.1:9630/api/apps'
