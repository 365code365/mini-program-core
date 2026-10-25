#!/usr/bin/env python3
"""把 sample/ 下的小程序打包上传到 mini-store（新建或升级，版本号 +1）。

    python3 tools/mini-store/upload_samples.py music train ...        # 指定示例目录名
    MINI_STORE_URL=http://host:9630 MINI_STORE_USER=admin MINI_STORE_PASS=… python3 tools/mini-store/upload_samples.py music

账户默认从 sample/测试服务器.txt 读（第 5 行管理地址、第 6 行用户名、第 7 行密码），环境变量优先。
密码不打印、不写日志。展示信息（名称 / 简介 / 分类 / 标签 / 开发者）取各示例的 store.meta.json，
AppID 取 project.config.json。只用标准库。
"""
import io
import json
import os
import sys
import urllib.parse
import urllib.request
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SKIP = {'.DS_Store'}


def account():
    url, user, pw = os.environ.get('MINI_STORE_URL'), os.environ.get('MINI_STORE_USER'), os.environ.get('MINI_STORE_PASS')
    path = os.path.join(ROOT, 'sample', '测试服务器.txt')
    if (not url or not user or not pw) and os.path.isfile(path):
        lines = [l.strip() for l in open(path, encoding='utf-8')]
        if len(lines) >= 7:
            u = urllib.parse.urlsplit(lines[4])
            url = url or f'{u.scheme}://{u.netloc}'
            user = user or lines[5]
            pw = pw or lines[6]
    if not (url and user and pw):
        sys.exit('没有商店账户：设 MINI_STORE_URL / MINI_STORE_USER / MINI_STORE_PASS')
    return url.rstrip('/'), user, pw


def call(method, url, body=None, headers=None):
    req = urllib.request.Request(url, data=body, method=method, headers=headers or {})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            return json.loads(r.read().decode('utf-8'))
    except urllib.error.HTTPError as e:
        raise SystemExit(f'{method} {urllib.parse.urlsplit(url).path} → {e.code} {e.read().decode("utf-8", "replace")[:300]}')


def pack(app_dir):
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', zipfile.ZIP_DEFLATED) as z:
        for base, dirs, files in os.walk(app_dir):
            dirs[:] = [d for d in dirs if not d.startswith('.')]
            for f in sorted(files):
                if f in SKIP:
                    continue
                full = os.path.join(base, f)
                z.write(full, os.path.relpath(full, app_dir))
    return buf.getvalue()


def main(names):
    url, user, pw = account()
    login = call('POST', url + '/api/login', json.dumps({'username': user, 'password': pw}).encode(),
                 {'Content-Type': 'application/json'})
    auth = {'Authorization': 'Bearer ' + login['token']}
    for name in names:
        d = os.path.join(ROOT, 'sample', name)
        meta = json.load(open(os.path.join(d, 'store.meta.json'), encoding='utf-8'))
        appid = json.load(open(os.path.join(d, 'project.config.json'), encoding='utf-8'))['appid']
        q = urllib.parse.urlencode({
            'name': meta['name'], 'description': meta.get('description', ''), 'appid': appid,
            'category': meta.get('category', '其他'), 'tags': ','.join(meta.get('tags', [])),
            'developer': meta.get('developer', ''),
        })
        body = pack(d)
        r = call('PUT', f'{url}/api/admin/apps/{urllib.parse.quote(name)}?{q}', body,
                 dict(auth, **{'Content-Type': 'application/zip'}))
        print(f"{name:12} {r['appId']}  v{r['version']}  {r['files']} 个文件  {r['size'] // 1024}KB  {r['name']}（{r.get('category')}）")


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    main(sys.argv[1:])
