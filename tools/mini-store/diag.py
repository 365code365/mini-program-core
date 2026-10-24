"""诊断日志：示例 App 里点「上传诊断日志」→ POST /api/diag，存到 `$MINI_STORE_DATA/diag/<id>.json`；
开发机（或 AI）用 tools/mini-store/diag.sh 拉回来分析。

为什么要这一条：真机上的问题现场多半不在开发机旁边（没插 USB、别人手里的手机），
adb logcat 拿不到；让 App 自己把本进程的日志打包传上来，拿到的是出问题那一刻的完整上下文。

上传是**公开**接口（App 里没有管理员账户）：限大小、限频率、只保留最近 MAX_KEEP 份；
查看 / 下载 / 删除要登录（日志里可能有页面数据，不公开）。
"""
import gzip
import json
import os
import re
import secrets
import threading
import time

from store_pkg import DATA, HttpError, write_json

MAX_BODY = 8 << 20          # 解压后的上限
MAX_KEEP = 500              # 只留最近这么多份
RATE_PER_HOUR = 30          # 每个 IP 每小时最多传几份
ID_RE = re.compile(r'^\d{8}-\d{6}-[0-9a-f]{6}$')
_LOCK = threading.Lock()
_RECENT = {}                # ip → [时刻…]


def _dir():
    d = os.path.join(DATA, 'diag')
    os.makedirs(d, exist_ok=True)
    return d


def _throttle(ip):
    now = time.time()
    with _LOCK:
        hits = [t for t in _RECENT.get(ip, []) if now - t < 3600]
        if len(hits) >= RATE_PER_HOUR:
            raise HttpError(429, '上传太频繁，稍后再试')
        hits.append(now)
        _RECENT[ip] = hits


def receive(raw, encoding, ip):
    """收一份：body 是 JSON（可 gzip）。返回 {id}"""
    _throttle(ip)
    if encoding == 'gzip':
        try:
            raw = gzip.decompress(raw)
        except OSError:
            raise HttpError(400, 'gzip 解不开')
    if len(raw) > MAX_BODY:
        raise HttpError(413, '日志太大')
    try:
        body = json.loads(raw.decode('utf-8', 'replace'))
    except ValueError:
        raise HttpError(400, '不是 JSON')
    if not isinstance(body, dict):
        raise HttpError(400, '要一个 JSON 对象')
    diag_id = time.strftime('%Y%m%d-%H%M%S') + '-' + secrets.token_hex(3)
    body['id'] = diag_id
    body['receivedAt'] = int(time.time() * 1000)
    body['ip'] = ip
    write_json(os.path.join(_dir(), diag_id + '.json'), body)
    _prune()
    return {'id': diag_id}


def _prune():
    files = sorted(f for f in os.listdir(_dir()) if f.endswith('.json'))
    for f in files[:-MAX_KEEP]:
        try:
            os.remove(os.path.join(_dir(), f))
        except OSError:
            pass


def _summary(diag_id, body, size):
    logs = body.get('logs') or ''
    return {
        'id': diag_id,
        'receivedAt': body.get('receivedAt'),
        'host': body.get('host'),
        'platform': body.get('platform'),
        'device': body.get('device'),
        'app': body.get('app'),
        'note': body.get('note'),
        'size': size,
        'logLines': logs.count('\n') + (1 if logs else 0),
    }


def list_all(limit=100):
    out = []
    for f in sorted((f for f in os.listdir(_dir()) if f.endswith('.json')), reverse=True)[:limit]:
        p = os.path.join(_dir(), f)
        try:
            with open(p, encoding='utf-8') as fh:
                out.append(_summary(f[:-5], json.load(fh), os.path.getsize(p)))
        except (OSError, ValueError):
            continue
    return out


def _path(diag_id):
    if diag_id == 'latest':
        files = sorted(f for f in os.listdir(_dir()) if f.endswith('.json'))
        if not files:
            raise HttpError(404, '还没有诊断日志')
        diag_id = files[-1][:-5]
    if not ID_RE.match(diag_id):
        raise HttpError(400, '诊断 id 不对')
    p = os.path.join(_dir(), diag_id + '.json')
    if not os.path.isfile(p):
        raise HttpError(404, '没有这份诊断日志')
    return p


def get(diag_id):
    with open(_path(diag_id), encoding='utf-8') as f:
        return json.load(f)


def delete(diag_id):
    os.remove(_path(diag_id))
