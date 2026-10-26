"""灵犀官网（介绍页 + 演示 App 下载）的静态服务：只用 Python 标准库，与 mini-store 同一台机器、另一个端口。

    MINI_SITE_ROOT=/opt/mini-site/www MINI_SITE_PORT=9640 python3 server.py

为什么不用 `python3 -m http.server`：
- 要给 .apk 正确的 Content-Type（`application/vnd.android.package-archive`），否则部分手机浏览器把它当 zip 存下、点了不能装；
- 要支持 Range（断点续传）：60MB 的包在手机网络上断一次就从头下，体验很差；
- 不列目录（http.server 默认把整个目录列出来），路径防穿越；
- 下载按 `download/<文件名>` 给 `Content-Disposition: attachment`，页面里的其它资源正常缓存。
"""
import email.utils
import os
import re
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import unquote, urlsplit

ROOT = os.path.abspath(os.environ.get('MINI_SITE_ROOT', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'www')))
PORT = int(os.environ.get('MINI_SITE_PORT', '9640'))
TYPES = {
    '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8', '.js': 'text/javascript; charset=utf-8',
    '.json': 'application/json; charset=utf-8', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg',
    '.webp': 'image/webp', '.svg': 'image/svg+xml', '.mp4': 'video/mp4', '.ico': 'image/x-icon',
    '.apk': 'application/vnd.android.package-archive', '.zip': 'application/zip', '.txt': 'text/plain; charset=utf-8',
}
RANGE_RE = re.compile(r'^bytes=(\d*)-(\d*)$')


class Handler(BaseHTTPRequestHandler):
    server_version = 'lingxi-site/1'

    def log_message(self, fmt, *args):
        sys.stderr.write(f'{self.client_address[0]} {fmt % args}\n')

    def do_HEAD(self):
        self.serve(head=True)

    def do_GET(self):
        self.serve(head=False)

    def resolve(self, path):
        """URL 路径 → 文件（防穿越；目录取 index.html）。下载区 `download/x.apk` 映射到 `downloads/x.apk`"""
        parts = [p for p in unquote(path).split('/') if p]
        if any(p in ('.', '..') or p.startswith('.') for p in parts):
            return None, False
        attach = bool(parts) and parts[0] == 'download'
        if attach:
            parts[0] = 'downloads'
        full = os.path.abspath(os.path.join(ROOT, *parts))
        if not (full == ROOT or full.startswith(ROOT + os.sep)):
            return None, False
        if os.path.isdir(full):
            if path and not path.endswith('/'):
                return ('redirect', path + '/'), False
            full = os.path.join(full, 'index.html')
        return (full if os.path.isfile(full) else None), attach

    def serve(self, head):
        url = urlsplit(self.path)
        full, attach = self.resolve(url.path)
        if isinstance(full, tuple):
            self.send_response(301)
            self.send_header('Location', full[1] + (('?' + url.query) if url.query else ''))
            self.send_header('Content-Length', '0')
            self.end_headers()
            return
        if not full:
            body = '没有这个页面'.encode('utf-8')
            self.send_response(404)
            self.send_header('Content-Type', 'text/plain; charset=utf-8')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            if not head:
                self.wfile.write(body)
            return
        st = os.stat(full)
        size = st.st_size
        ext = os.path.splitext(full)[1].lower()
        start, end, status = 0, size - 1, 200
        m = RANGE_RE.match(self.headers.get('Range', '').strip())
        if m and size > 0:
            a, b = m.groups()
            if a:
                start, end = int(a), min(int(b), size - 1) if b else size - 1
            elif b:
                start, end = max(0, size - int(b)), size - 1
            if start > end or start >= size:
                self.send_response(416)
                self.send_header('Content-Range', f'bytes */{size}')
                self.send_header('Content-Length', '0')
                self.end_headers()
                return
            status = 206
        self.send_response(status)
        self.send_header('Content-Type', TYPES.get(ext, 'application/octet-stream'))
        self.send_header('Content-Length', str(end - start + 1))
        self.send_header('Accept-Ranges', 'bytes')
        self.send_header('Last-Modified', email.utils.formatdate(st.st_mtime, usegmt=True))
        if status == 206:
            self.send_header('Content-Range', f'bytes {start}-{end}/{size}')
        if attach:
            self.send_header('Content-Disposition', f'attachment; filename="{os.path.basename(full)}"')
            self.send_header('Cache-Control', 'no-cache')
        elif ext in ('.html', '.json'):
            self.send_header('Cache-Control', 'no-cache')
        else:
            self.send_header('Cache-Control', 'public, max-age=3600')
        self.end_headers()
        if head:
            return
        with open(full, 'rb') as f:
            f.seek(start)
            left = end - start + 1
            try:
                while left > 0:
                    chunk = f.read(min(1 << 16, left))
                    if not chunk:
                        break
                    self.wfile.write(chunk)
                    left -= len(chunk)
            except (BrokenPipeError, ConnectionResetError):
                pass


def main():
    print(f'灵犀官网 {ROOT} → http://0.0.0.0:{PORT}', flush=True)
    ThreadingHTTPServer(('0.0.0.0', PORT), Handler).serve_forever()


if __name__ == '__main__':
    main()
