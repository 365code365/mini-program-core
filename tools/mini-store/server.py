#!/usr/bin/env python3
"""小程序管理服务（mini-store）：上传 / 列表 / 下载小程序包，示例 App 从这里拉列表、点开即下载。

只用 Python 标准库（服务器上不装依赖）。数据都在 `$MINI_STORE_DATA`（默认 /opt/mini-store/data）：

    config.json               账户（PBKDF2 哈希）与会话签名密钥
    apps/<id>/meta.json       AppID、名称、描述、版本号、sha256、大小……
    apps/<id>/package.zip     规整过的包：app.json（小程序）或 game.json（小游戏）在 zip 根上，
                              project.config.json 里的 appid 与 meta 一致、compileType 与包的种类一致

包的校验、AppID 分配见 store_pkg.py；H5 落地页见 landing.py。

接口（<key> = AppID 或 id）：
    GET    /api/apps?q=&category=&limit= 公开：小程序列表 / 搜索（示例 App 用；q 为空 = 全部，按热度）
           &kind=miniprogram|game|all    包的种类，不给 = 只要小程序（现有 App 只会开 app.json 的包）；
                                         每条带 kind / kindLabel，小游戏另带 orientation
           &page=&pageSize=              给了任一个才分页（pageSize 1~100，默认 20），回 {apps, total, page, pageSize}；
                                         不给保持全量（App 依赖全量列表）
    GET    /api/demo/…                   公开：示例小程序的演示数据（CORS *；完整说明见 demo_api.py 文件头）
           列表都收 ?page=1&size=10（size ≤ 50），回 {list, page, size, total, hasMore}：
           movies?type=now|coming|top&q=   movies/<id>   movies/<id>/comments?sort=hot|new
           banners?app=movie|music|gallery   tags?app=gallery|music|movie
           music/playlists?tag=   music/playlists/<id>   music/songs?q=   music/songs/<id>（lrc + audioUrl）   music/charts
           photos?tag=&q=   photos/<id>   img/<kind>/<name>.jpg   audio/<n>.wav
           图片 / 音频由 make_demo_assets.py 在本机生成到 demo_assets/，随 deploy.sh 上传
    GET    /api/categories?kind=         公开：分类与各分类的数量（kind 同上）
    GET    /api/search/hot               公开：热搜词
    GET    /api/search/suggest?q=        公开：输入联想
    GET    /api/apps/<key>/icon          公开：商店图标（包根上的 icon.png）
    GET    /api/apps/<key>               公开：一个小程序的最新版本信息（App 打开前据此判断要不要更新）
    GET    /api/apps/<key>/package       公开：下载包（ETag = sha256，带 If-None-Match 回 304）
    GET    /open?appid=&path=&query=     公开：H5 跳小程序的落地页（拉起 App 的 URL Scheme）
    GET    /a/<AppID>?path=&query=       同上，短链接
    POST   /api/diag                     公开：App 上传诊断日志（JSON，可 gzip；限大小限频，见 diag.py）
    GET    /api/admin/diag               诊断日志列表（最新在前）
    GET    /api/admin/diag/<id|latest>   一份诊断日志的全文
    DELETE /api/admin/diag/<id>
    POST   /api/login                    {"username","password"} → 设 Cookie，同时返回 token（脚本用 Bearer）
    POST   /api/logout
    GET    /api/admin/me                 登录态
    PUT    /api/admin/apps/<id>?name=&description=&appid=&category=&tags=&developer=   body = zip，新建或升级（版本号 +1）
    PATCH  /api/admin/apps/<id>          {"name","description","category","tags","developer","order"} 只改展示信息
    DELETE /api/admin/apps/<id>
    GET    /admin                        管理页面

用法：
    python3 server.py set-password <用户名>   # 从标准输入读密码；会换掉签名密钥（已登录的全部失效）
    python3 server.py serve                   # 端口 $MINI_STORE_PORT（默认 9630）
"""
import base64
import hashlib
import hmac
import json
import os
import secrets
import shutil
import socketserver
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit

import demo_api
import diag
import landing
import store_search
from store_pkg import (DATA, HttpError, app_dir, bump_downloads, delete_app, find, kind_filter, list_apps, patch_app,
                       public_meta, save_app, write_json)

PORT = int(os.environ.get('MINI_STORE_PORT', '9630'))
STATIC = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'static')
MAX_UPLOAD = 128 << 20      # 上传的 zip
SESSION_TTL = 12 * 3600
PBKDF2_ROUNDS = 240_000
# ─────────────────────────── 账户与会话 ───────────────────────────

def config_path():
    return os.path.join(DATA, 'config.json')


def load_config():
    try:
        with open(config_path(), encoding='utf-8') as f:
            return json.load(f)
    except FileNotFoundError:
        sys.exit(f'没有 {config_path()}：先跑 `python3 server.py set-password <用户名>`')


def set_password(username):
    pw = sys.stdin.readline().rstrip('\n')
    if len(pw) < 8:
        sys.exit('密码至少 8 位')
    salt = secrets.token_hex(16)
    os.makedirs(DATA, mode=0o700, exist_ok=True)
    write_json(config_path(), {
        'username': username,
        'salt': salt,
        'rounds': PBKDF2_ROUNDS,
        'hash': hash_pw(pw, salt, PBKDF2_ROUNDS),
        'secret': secrets.token_hex(32),
    })
    os.chmod(config_path(), 0o600)
    print(f'已设置账户 {username}')


def hash_pw(pw, salt, rounds):
    return hashlib.pbkdf2_hmac('sha256', pw.encode('utf-8'), bytes.fromhex(salt), rounds).hex()


def sign(cfg, payload):
    return hmac.new(bytes.fromhex(cfg['secret']), payload.encode(), hashlib.sha256).hexdigest()


def make_token(cfg):
    payload = f"{cfg['username']}|{int(time.time()) + SESSION_TTL}"
    return base64.urlsafe_b64encode(payload.encode()).decode().rstrip('=') + '.' + sign(cfg, payload)


def check_token(cfg, token):
    try:
        b64, sig = token.split('.', 1)
        payload = base64.urlsafe_b64decode(b64 + '=' * (-len(b64) % 4)).decode()
        user, exp = payload.rsplit('|', 1)
    except (ValueError, UnicodeDecodeError):
        return False
    return hmac.compare_digest(sig, sign(cfg, payload)) and user == cfg['username'] and int(exp) > time.time()


class LoginThrottle:
    """同一 IP 连错 5 次锁 5 分钟（密码走的是明文 HTTP，至少别让人在线爆破）"""

    def __init__(self):
        self.fails = {}
        self.lock = threading.Lock()

    def blocked(self, ip):
        with self.lock:
            n, until = self.fails.get(ip, (0, 0))
            return n >= 5 and until > time.time()

    def record(self, ip, ok):
        with self.lock:
            if ok:
                self.fails.pop(ip, None)
                return
            n, until = self.fails.get(ip, (0, 0))
            if until and until <= time.time():
                n = 0
            self.fails[ip] = (n + 1, time.time() + 300)


THROTTLE = LoginThrottle()


def _int_arg(v, default, lo, hi):
    try:
        return max(lo, min(hi, int(v)))
    except (TypeError, ValueError):
        return default


# ─────────────────────────── HTTP ───────────────────────────

class Handler(BaseHTTPRequestHandler):
    server_version = 'mini-store/1'
    cfg = None

    def log_message(self, fmt, *args):
        sys.stderr.write(f'{self.client_address[0]} {fmt % args}\n')

    # 入口：统一捕获 HttpError
    def do_GET(self):
        self.dispatch('GET')

    def do_POST(self):
        self.dispatch('POST')

    def do_PUT(self):
        self.dispatch('PUT')

    def do_PATCH(self):
        self.dispatch('PATCH')

    def do_DELETE(self):
        self.dispatch('DELETE')

    def do_OPTIONS(self):
        # 只有 /api/demo/ 回 CORS 预检，其它路径照旧 404
        self.dispatch('OPTIONS')

    def dispatch(self, method):
        url = urlsplit(self.path)
        self.query = {k: v[0] for k, v in parse_qs(url.query).items()}
        parts = [p for p in url.path.split('/') if p]
        try:
            self.route(method, parts)
        except HttpError as e:
            self.send_json({'error': e.message}, e.status)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def route(self, method, p):
        if method == 'GET' and p in ([], ['admin']):
            return self.send_static('admin.html', 'text/html; charset=utf-8')
        if method == 'GET' and p in (['admin.js'], ['open.js'], ['pager.js']):
            return self.send_static(p[0], 'text/javascript; charset=utf-8')
        if method == 'GET' and (p == ['open'] or (len(p) == 2 and p[0] == 'a')):
            appid = p[1] if len(p) == 2 else self.query.get('appid', '')
            status, page = landing.render(appid, self.query.get('path', ''), self.query.get('query', ''))
            return self.send_html(page, status)
        if method == 'GET' and p == ['api', 'categories']:
            return self.send_json({'categories': store_search.categories(kind_filter(self.query.get('kind')))})
        if method == 'GET' and p == ['api', 'search', 'hot']:
            return self.send_json({'keywords': store_search.hot_keywords()})
        if method == 'GET' and p == ['api', 'search', 'suggest']:
            return self.send_json({'words': store_search.suggest(self.query.get('q', ''))})
        if p[:2] == ['api', 'apps']:
            if method == 'GET' and len(p) == 2:
                return self.list_apps_api()
            if method == 'GET' and len(p) == 4 and p[3] == 'icon':
                return self.send_icon(p[2])
            if method == 'GET' and len(p) == 3:
                meta = find(p[2])
                if not meta:
                    raise HttpError(404, '没有这个小程序')
                return self.send_json(dict(public_meta(meta), serverTime=int(time.time() * 1000)))
            if method == 'GET' and len(p) == 4 and p[3] == 'package':
                return self.send_package(p[2])
        if p == ['api', 'diag'] and method == 'POST':
            raw = self.read_body(diag.MAX_BODY)
            return self.send_json(diag.receive(raw, self.headers.get('Content-Encoding', ''), self.client_address[0]))
        if p == ['api', 'login'] and method == 'POST':
            return self.login()
        if p == ['api', 'logout'] and method == 'POST':
            return self.send_json({'ok': True}, cookie='ms_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict')
        if p[:2] == ['api', 'admin']:
            self.require_auth(method)
            if p == ['api', 'admin', 'me'] and method == 'GET':
                return self.send_json({'username': self.cfg['username']})
            if p == ['api', 'admin', 'diag'] and method == 'GET':
                return self.send_json({'items': diag.list_all()})
            if len(p) == 4 and p[2] == 'diag':
                if method == 'GET':
                    return self.send_json(diag.get(p[3]))
                if method == 'DELETE':
                    diag.delete(p[3])
                    return self.send_json({'ok': True})
            if len(p) == 4 and p[2] == 'apps':
                if method == 'PUT':
                    body = self.read_body(MAX_UPLOAD)
                    meta = save_app(p[3], body, self.query.get('name', '').strip()[:200],
                                    self.query.get('description'), self.query.get('appid'),
                                    {k: self.query[k] for k in ('category', 'tags', 'developer') if k in self.query})
                    return self.send_json(meta)
                if method == 'PATCH':
                    return self.send_json(patch_app(p[3], self.read_json()))
                if method == 'DELETE':
                    delete_app(p[3])
                    return self.send_json({'ok': True})
        if p[:2] == ['api', 'demo']:
            return demo_api.handle(self, method, p[2:], self.query)
        raise HttpError(404, '没有这个接口')

    def list_apps_api(self):
        """GET /api/apps：分页参数给了才分页 —— App 拿的是全量（不带参数），管理页按页拉"""
        q, cat = self.query.get('q', ''), self.query.get('category', '')
        kinds = kind_filter(self.query.get('kind'))
        paged = 'page' in self.query or 'pageSize' in self.query
        page = _int_arg(self.query.get('page'), 1, 1, 100_000)
        size = _int_arg(self.query.get('pageSize'), 20, 1, 100)
        now = int(time.time() * 1000)
        if not q and not cat:
            apps = list_apps(kinds)
            if not paged:
                return self.send_json({'apps': apps, 'serverTime': now})
            return self.send_json({'apps': apps[(page - 1) * size:page * size], 'total': len(apps), 'page': page,
                                   'pageSize': size, 'serverTime': now})
        if paged:
            apps, total = store_search.search(q, cat, size, offset=(page - 1) * size, kinds=kinds)
            return self.send_json({'apps': apps, 'total': total, 'page': page, 'pageSize': size, 'q': q,
                                   'category': cat, 'serverTime': now})
        apps, total = store_search.search(q, cat, _int_arg(self.query.get('limit'), 50, 1, 200), kinds=kinds)
        return self.send_json({'apps': apps, 'total': total, 'q': q, 'category': cat, 'serverTime': now})

    # ── 鉴权 ──
    def login(self):
        ip = self.client_address[0]
        if THROTTLE.blocked(ip):
            raise HttpError(429, '错误次数太多，5 分钟后再试')
        body = self.read_json()
        cfg = self.cfg
        user_ok = hmac.compare_digest(str(body.get('username', '')), cfg['username'])
        pw_ok = hmac.compare_digest(hash_pw(str(body.get('password', '')), cfg['salt'], cfg['rounds']), cfg['hash'])
        THROTTLE.record(ip, user_ok and pw_ok)
        if not (user_ok and pw_ok):
            raise HttpError(401, '账户或密码不对')
        token = make_token(cfg)
        cookie = f'ms_session={token}; Path=/; Max-Age={SESSION_TTL}; HttpOnly; SameSite=Strict'
        self.send_json({'ok': True, 'token': token, 'username': cfg['username']}, cookie=cookie)

    def require_auth(self, method):
        auth = self.headers.get('Authorization', '')
        if auth.startswith('Bearer ') and check_token(self.cfg, auth[7:].strip()):
            return
        cookies = dict(c.strip().split('=', 1) for c in self.headers.get('Cookie', '').split(';') if '=' in c)
        if check_token(self.cfg, cookies.get('ms_session', '')):
            # Cookie 会被浏览器自动带上：改数据的请求另要一个自定义头（跨站表单加不了），挡 CSRF
            if method != 'GET' and self.headers.get('X-Mini-Store') != '1':
                raise HttpError(403, '缺少 X-Mini-Store 头')
            return
        raise HttpError(401, '请先登录')

    # ── 读写 ──
    def read_body(self, limit):
        try:
            n = int(self.headers.get('Content-Length', ''))
        except ValueError:
            raise HttpError(411, '要 Content-Length')
        if n > limit:
            raise HttpError(413, f'太大（上限 {limit >> 20} MB）')
        buf = bytearray()
        while len(buf) < n:
            chunk = self.rfile.read(min(1 << 20, n - len(buf)))
            if not chunk:
                raise HttpError(400, '请求体不完整')
            buf += chunk
        return bytes(buf)

    def read_json(self):
        try:
            body = json.loads(self.read_body(64 << 10) or b'{}')
        except ValueError:
            raise HttpError(400, '不是 JSON')
        if not isinstance(body, dict):
            raise HttpError(400, '要一个 JSON 对象')
        return body

    def send_json(self, obj, status=200, cookie=None):
        data = json.dumps(obj, ensure_ascii=False).encode('utf-8')
        self.send_response(status)
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', 'no-store')
        if cookie:
            self.send_header('Set-Cookie', cookie)
        self.end_headers()
        if self.command != 'HEAD':
            self.wfile.write(data)

    def send_html(self, page, status=200):
        data = page.encode('utf-8')
        self.send_response(status)
        self.send_header('Content-Type', 'text/html; charset=utf-8')
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', 'no-cache')
        self.send_header('Content-Security-Policy', "default-src 'self'; style-src 'self' 'unsafe-inline'")
        self.end_headers()
        self.wfile.write(data)

    def send_static(self, name, ctype):
        with open(os.path.join(STATIC, name), 'rb') as f:
            data = f.read()
        self.send_response(200)
        self.send_header('Content-Type', ctype)
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', 'no-cache')
        self.send_header('Content-Security-Policy',
                         "default-src 'self'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'")
        self.send_header('X-Content-Type-Options', 'nosniff')
        self.end_headers()
        self.wfile.write(data)

    def send_package(self, key):
        meta = find(key)
        if not meta:
            raise HttpError(404, '没有这个小程序')
        path = os.path.join(app_dir(meta['id']), 'package.zip')
        if not meta or not os.path.isfile(path):
            raise HttpError(404, '没有这个小程序')
        etag = f'"{meta["sha256"]}"'
        if self.headers.get('If-None-Match') == etag:
            self.send_response(304)
            self.send_header('ETag', etag)
            self.end_headers()
            return
        size = os.path.getsize(path)
        bump_downloads(meta['id'])
        self.send_response(200)
        self.send_header('Content-Type', 'application/zip')
        self.send_header('Content-Length', str(size))
        self.send_header('ETag', etag)
        self.send_header('X-Sha256', meta['sha256'])
        self.send_header('Cache-Control', 'no-cache')
        self.end_headers()
        with open(path, 'rb') as f:
            shutil.copyfileobj(f, self.wfile, 256 << 10)


    def send_icon(self, key):
        meta = find(key)
        path = os.path.join(app_dir(meta['id']), 'icon.png') if meta else ''
        if not meta or not os.path.isfile(path):
            raise HttpError(404, '没有图标')
        with open(path, 'rb') as f:
            data = f.read()
        self.send_response(200)
        self.send_header('Content-Type', 'image/png')
        self.send_header('Content-Length', str(len(data)))
        # 地址带 ?v=版本，换包就换地址：可以长缓存
        self.send_header('Cache-Control', 'public, max-age=86400')
        self.end_headers()
        self.wfile.write(data)


class _Server(ThreadingHTTPServer):
    """`HTTPServer.server_bind` 会 `socket.getfqdn()` 反查本机名：网络环境里反查超时（2026-10-26 开发机实测卡住十几秒），
    服务就迟迟不监听，e2e 的 10 秒等待（tests/sample_harness/demo_server.rs）全部超时。server_name 只用于 CGI 环境变量，不反查"""

    def server_bind(self):
        socketserver.TCPServer.server_bind(self)
        host, port = self.server_address[:2]
        self.server_name = host
        self.server_port = port


def serve():
    Handler.cfg = load_config()
    os.makedirs(os.path.join(DATA, 'apps'), exist_ok=True)
    httpd = _Server(('0.0.0.0', PORT), Handler)
    httpd.daemon_threads = True
    print(f'mini-store 在 :{PORT}，数据 {DATA}', flush=True)
    httpd.serve_forever()


if __name__ == '__main__':
    if len(sys.argv) == 3 and sys.argv[1] == 'set-password':
        set_password(sys.argv[2])
    elif len(sys.argv) == 2 and sys.argv[1] == 'serve':
        serve()
    else:
        sys.exit(__doc__)
