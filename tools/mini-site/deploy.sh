#!/usr/bin/env bash
# 部署灵犀官网（介绍页 + 演示 App 下载）到服务器：http://<服务器>:9640/
#
#   bash tools/mini-site/deploy.sh [Flutter 包.apk] [原生包.apk]
#
# 不给包就用默认的构建产物（先构建好）：
#   cd sdk/flutter/example && flutter build apk --release --target-platform android-arm64
#   cd sdk/android/example && ./gradlew assembleRelease
# 包改名成 lingxi-flutter-<版本>.apk / lingxi-native-<版本>.apk 放进 downloads/，生成 downloads.json（大小、sha256、时间）。
# site/（英文为主、可切中文的完整技术介绍）一起放到 /intro/。
#
# MINI_SITE_HOST（默认 root@64.83.22.217）；ssh 要密码时先 `export SSHPASS=…`，脚本会走 sshpass。
# 端口 9640：服务器上 22 / 80 / 888 / 8889 / 8899 / 8900 / 9630（商店）都占着，9640 没人用。
set -euo pipefail
cd "$(dirname "$0")/../.."
HOST="${MINI_SITE_HOST:-root@64.83.22.217}"
PORT=9640
FLUTTER_APK="${1:-sdk/flutter/example/build/app/outputs/flutter-apk/app-release.apk}"
NATIVE_APK="${2:-sdk/android/example/app/build/outputs/apk/release/app-release.apk}"
for f in "$FLUTTER_APK" "$NATIVE_APK"; do [ -f "$f" ] || { echo "❌ 没有 $f（先构建）"; exit 1; }; done
if [ -n "${SSHPASS:-}" ]; then S=(sshpass -e); else S=(); fi
R() { "${S[@]}" ssh -o ConnectTimeout=15 "$HOST" "$@"; }

VER_F=$(sed -n 's/^version: *\([^+ ]*\).*/\1/p' sdk/flutter/example/pubspec.yaml)
VER_N=$(sed -n 's/.*versionName *= *"\(.*\)".*/\1/p' sdk/android/example/app/build.gradle.kts)
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
W="$STAGE/www"
mkdir -p "$W/downloads" "$W/shots" "$W/intro"
cp tools/mini-site/index.html tools/mini-site/style.css tools/mini-site/app.js "$W/"
cp tools/mini-site/shots/* "$W/shots/"
cp -R site/. "$W/intro/"
cp "$FLUTTER_APK" "$W/downloads/lingxi-flutter-$VER_F.apk"
cp "$NATIVE_APK" "$W/downloads/lingxi-native-$VER_N.apk"
python3 - "$W" "$VER_F" "$VER_N" <<'EOF'
import hashlib, json, os, sys, time
w, vf, vn = sys.argv[1:4]
def meta(file, title, icon, note, ver):
    p = os.path.join(w, 'downloads', file)
    h = hashlib.sha256(open(p, 'rb').read()).hexdigest()
    return {'file': file, 'title': title, 'icon': icon, 'note': note, 'version': ver,
            'size': os.path.getsize(p), 'sha256': h, 'date': time.strftime('%Y-%m-%d')}
files = [
    meta(f'lingxi-flutter-{vf}.apk', '灵犀（Flutter 版）', 'shots/icon-flutter.png',
         'Flutter 插件 mini_render 的完整示例：下拉小程序面板、收藏、商店、胶囊菜单。', vf),
    meta(f'lingxi-native-{vn}.apk', '灵犀原生（Android 原生版）', 'shots/icon-native.png',
         '纯 Android 原生集成（MiniProgramView），不依赖 AndroidX。', vn),
]
json.dump({'files': files, 'updatedAt': int(time.time() * 1000)}, open(os.path.join(w, 'downloads.json'), 'w'), ensure_ascii=False, indent=1)
for f in files:
    print(f"  {f['file']}  {f['size'] / 1048576:.1f}MB  {f['sha256'][:16]}…")
EOF
cp tools/mini-site/server.py tools/mini-site/mini-site.service "$STAGE/"

# 先传到临时目录、整个换上去：传一半断掉不会留下半个网站
COPYFILE_DISABLE=1 tar --no-xattrs -czf - -C "$STAGE" www server.py mini-site.service | R 'set -e
id mini-store >/dev/null 2>&1 || useradd --system --home /opt/mini-store --shell /usr/sbin/nologin mini-store
rm -rf /opt/mini-site.new && mkdir -p /opt/mini-site.new
tar xzf - -C /opt/mini-site.new
mv /opt/mini-site.new/mini-site.service /etc/systemd/system/mini-site.service
chown -R root:root /opt/mini-site.new && chmod -R a+rX /opt/mini-site.new
rm -rf /opt/mini-site.old; [ -d /opt/mini-site ] && mv /opt/mini-site /opt/mini-site.old; mv /opt/mini-site.new /opt/mini-site
rm -rf /opt/mini-site.old'
R "set -e
systemctl daemon-reload
systemctl enable mini-site >/dev/null 2>&1
systemctl restart mini-site
command -v ufw >/dev/null && ufw allow $PORT/tcp >/dev/null
sleep 1
systemctl is-active mini-site
curl -sS -o /dev/null -w '本机 / → %{http_code}\n' http://127.0.0.1:$PORT/
curl -sS -o /dev/null -w '本机 downloads.json → %{http_code}\n' http://127.0.0.1:$PORT/downloads.json"
echo "✅ http://${HOST#*@}:$PORT/"
