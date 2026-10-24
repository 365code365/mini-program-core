"""mini-store 的小程序包：校验 / 规整上传的 zip、分配 AppID、存取 meta。

AppID 是小程序的对外身份（深链、H5 跳转、`wx.navigateToMiniProgram` 都按它找），
id 只是存储目录名。AppID 的来源，按优先级：
  1. 上传时显式给的 `appid` 参数；
  2. 包里 `project.config.json` 的 `appid`（`touristappid` 是开发者工具的游客占位，不算）；
  3. 这个 id 以前分配过的；
  4. 都没有就新分配一个 `wx` + 16 位十六进制（与微信 AppID 同一形状）。
最终的 AppID 会**写回包里的 project.config.json** —— 引擎从那里读
（`wx.getAccountInfoSync().miniProgram.appId`、胶囊「关于」），两边永远一致。
"""
import hashlib
import io
import json
import os
import re
import secrets
import shutil
import threading
import time
import zipfile

DATA = os.environ.get('MINI_STORE_DATA', '/opt/mini-store/data')
MAX_UNPACKED = 512 << 20    # 解开后的总大小（防 zip 炸弹）
MAX_FILES = 20000
# id 当目录名用（服务器 apps/<id>，客户端 files/mini-apps/<id>；引擎的 storage 也按目录名分），只许小写 ASCII
ID_RE = re.compile(r'^[a-z0-9][a-z0-9._-]{0,63}$')
APPID_RE = re.compile(r'^wx[0-9a-f]{16}$')
SKIP_NAMES = {'.DS_Store', 'Thumbs.db'}
# 分类是固定的一组（商店首页按它分栏、搜索页按它筛）：与示例小程序 store.meta.json 的 category 同一组
CATEGORIES = ['美食', '购物', '出行', '生活服务', '医疗健康', '金融', '工具', '教育', '娱乐', '资讯', '政务', '其他']
ICON_NAME = 'icon.png'          # 包根上的商店图标（144×144 PNG），上传时取出来单独存
MAX_ICON = 512 << 10
WRITE_LOCK = threading.Lock()


class HttpError(Exception):
    def __init__(self, status, message):
        super().__init__(message)
        self.status = status
        self.message = message


def write_json(path, obj):
    tmp = f'{path}.tmp{os.getpid()}'
    with open(tmp, 'w', encoding='utf-8') as f:
        json.dump(obj, f, ensure_ascii=False, indent=1)
    os.replace(tmp, path)


def app_dir(app_id):
    if not ID_RE.match(app_id):
        raise HttpError(400, 'id 只能是小写字母、数字、. _ -（1~64 位，字母或数字开头）')
    return os.path.join(DATA, 'apps', app_id)


def load_meta(app_id):
    try:
        with open(os.path.join(app_dir(app_id), 'meta.json'), encoding='utf-8') as f:
            return json.load(f)
    except FileNotFoundError:
        return None


def public_meta(meta):
    """对外的 meta：带上下载地址与图标地址（`?v=` 让中间缓存按版本区分）、累计打开次数"""
    out = dict(meta, packageUrl=f"/api/apps/{meta['id']}/package?v={meta['version']}")
    out.setdefault('category', '其他')
    out.setdefault('tags', [])
    out.setdefault('developer', '')
    if meta.get('hasIcon'):
        out['iconUrl'] = f"/api/apps/{meta['id']}/icon?v={meta['version']}"
    out['downloads'] = load_stats(meta['id']).get('downloads', 0)
    return out


def load_stats(app_id):
    try:
        with open(os.path.join(app_dir(app_id), 'stats.json'), encoding='utf-8') as f:
            return json.load(f)
    except (FileNotFoundError, ValueError):
        return {}


STATS_LOCK = threading.Lock()


def bump_downloads(app_id):
    """包被下载了一次（新装或升级）：热门排序用。与 meta 分开存，改它不碰包的版本"""
    with STATS_LOCK:
        st = load_stats(app_id)
        st['downloads'] = st.get('downloads', 0) + 1
        write_json(os.path.join(app_dir(app_id), 'stats.json'), st)


def clean_store_fields(body, old=None):
    """category / tags / developer：上传参数或 PATCH 里给的，校验后返回要写进 meta 的字段"""
    out = {}
    cat = body.get('category')
    if isinstance(cat, str) and cat.strip():
        if cat.strip() not in CATEGORIES:
            raise HttpError(400, f"category 只能是：{'、'.join(CATEGORIES)}")
        out['category'] = cat.strip()
    tags = body.get('tags')
    if isinstance(tags, str):
        tags = [t for t in tags.replace('，', ',').split(',')]
    if isinstance(tags, list):
        out['tags'] = [str(t).strip()[:20] for t in tags if str(t).strip()][:12]
    dev = body.get('developer')
    if isinstance(dev, str):
        out['developer'] = dev.strip()[:60]
    return out


def all_metas():
    root = os.path.join(DATA, 'apps')
    out = []
    for name in sorted(os.listdir(root)) if os.path.isdir(root) else []:
        meta = load_meta(name) if ID_RE.match(name) else None
        if meta:
            out.append(meta)
    out.sort(key=lambda m: (m.get('order', 1000), m['id']))
    return out


def list_apps():
    return [public_meta(m) for m in all_metas()]


def find(key):
    """按 AppID 或 id 找；找不到返回 None"""
    if APPID_RE.match(key):
        return next((m for m in all_metas() if m.get('appId') == key), None)
    return load_meta(key) if ID_RE.match(key) else None


def normalize_zip(data):
    """校验上传的 zip。返回 (条目列表 [(相对路径, 字节)], 文件数, 解开后大小, app.json, project.config.json 或 None)。

    允许两种布局：app.json 在根上；或整个小程序在唯一一层子目录里（Finder「压缩」出来就是这样）。
    拒绝绝对路径与 `..`（客户端照这份解包，路径穿越就是写到沙盒外面）"""
    try:
        zf = zipfile.ZipFile(io.BytesIO(data))
    except zipfile.BadZipFile:
        raise HttpError(400, '不是 zip 文件')
    entries = []
    for info in zf.infolist():
        if info.is_dir():
            continue
        name = info.filename.replace('\\', '/')
        parts = name.split('/')
        if parts[0] == '__MACOSX' or parts[-1] in SKIP_NAMES or parts[-1].startswith('._'):
            continue
        if name.startswith('/') or '..' in parts or ':' in parts[0] or '' in parts:
            raise HttpError(400, f'包里有非法路径：{name}')
        entries.append((name, info))
    names = {n for n, _ in entries}
    if 'app.json' in names:
        prefix = ''
    else:
        tops = {n.split('/')[0] for n in names}
        if len(tops) != 1 or f'{next(iter(tops))}/app.json' not in names:
            raise HttpError(400, '包里没有 app.json（要在 zip 根上，或在唯一一层子目录里）')
        prefix = next(iter(tops)) + '/'
    app_json = _read_json(zf, prefix + 'app.json', 'app.json')
    if not isinstance(app_json, dict) or not app_json.get('pages'):
        raise HttpError(400, 'app.json 里没有 pages')
    project = _read_json(zf, prefix + 'project.config.json', 'project.config.json') \
        if prefix + 'project.config.json' in names else None
    out, total = [], 0
    for name, info in entries:
        if not name.startswith(prefix):
            continue
        total += info.file_size
        if total > MAX_UNPACKED or len(out) >= MAX_FILES:
            raise HttpError(413, '包解开后太大')
        out.append((name[len(prefix):], info.date_time, zf.read(info)))
    return out, len(out), total, app_json, project if isinstance(project, dict) else None


def _read_json(zf, path, label):
    try:
        return json.loads(zf.read(path).decode('utf-8-sig'))
    except (ValueError, UnicodeDecodeError):
        raise HttpError(400, f'{label} 不是合法的 JSON')


def _pick_appid(app_id, explicit, project, old):
    if explicit:
        if not APPID_RE.match(explicit):
            raise HttpError(400, 'appid 要是 wx 加 16 位十六进制小写（如 wx3b7d9f1a2c4e6b80）')
        chosen = explicit
    else:
        from_pkg = str((project or {}).get('appid') or '').strip().lower()
        if APPID_RE.match(from_pkg):
            chosen = from_pkg
        elif old.get('appId'):
            chosen = old['appId']
        else:
            chosen = 'wx' + secrets.token_hex(8)
    owner = next((m for m in all_metas() if m.get('appId') == chosen and m['id'] != app_id), None)
    if owner:
        raise HttpError(409, f"AppID {chosen} 已被「{owner['name']}」（{owner['id']}）占用")
    return chosen


def save_app(app_id, data, name, description, appid=None, store_fields=None):
    entries, files, unpacked, app_json, project = normalize_zip(data)
    icon = next((c for rel, _, c in entries if rel == ICON_NAME), None)
    if icon is not None and (len(icon) > MAX_ICON or not icon.startswith(b'\x89PNG')):
        raise HttpError(400, 'icon.png 要是 PNG、不超过 512KB')
    d = app_dir(app_id)
    with WRITE_LOCK:
        old = load_meta(app_id) or {}
        chosen = _pick_appid(app_id, (appid or '').strip().lower(), project, old)
        title = (app_json.get('window') or {}).get('navigationBarTitleText')
        final_name = name or old.get('name') or (project or {}).get('projectname') or title or app_id
        # AppID 写回 project.config.json（保留包里原有的其它字段）
        proj = dict(project or {}, appid=chosen)
        proj.setdefault('projectname', app_id)
        proj_bytes = json.dumps(proj, ensure_ascii=False, indent=2).encode('utf-8')
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as w:
            for rel, dt, content in entries:
                if rel == 'project.config.json':
                    continue
                zi = zipfile.ZipInfo(rel, date_time=dt)
                zi.compress_type = zipfile.ZIP_DEFLATED
                w.writestr(zi, content)
            zi = zipfile.ZipInfo('project.config.json', date_time=time.localtime()[:6])
            zi.compress_type = zipfile.ZIP_DEFLATED
            w.writestr(zi, proj_bytes)
        pkg = buf.getvalue()
        os.makedirs(d, exist_ok=True)
        tmp = os.path.join(d, f'package.zip.tmp{os.getpid()}')
        with open(tmp, 'wb') as f:
            f.write(pkg)
        os.replace(tmp, os.path.join(d, 'package.zip'))
        if icon is not None:
            with open(os.path.join(d, 'icon.png'), 'wb') as f:
                f.write(icon)
        meta = {
            'id': app_id,
            'appId': chosen,
            'name': final_name,
            'description': description if description is not None else old.get('description', ''),
            'version': old.get('version', 0) + 1,
            'sha256': hashlib.sha256(pkg).hexdigest(),
            'size': len(pkg),
            'unpackedSize': unpacked,
            'files': files + (0 if project is not None else 1),
            'pages': len(app_json.get('pages') or []),
            'updatedAt': int(time.time() * 1000),
            'order': old.get('order', 1000),
            'hasIcon': icon is not None or (old.get('hasIcon', False) and os.path.isfile(os.path.join(d, 'icon.png'))),
            'category': old.get('category', '其他'),
            'tags': old.get('tags', []),
            'developer': old.get('developer', ''),
            'createdAt': old.get('createdAt', int(time.time() * 1000)),
        }
        meta.update(clean_store_fields(store_fields or {}))
        write_json(os.path.join(d, 'meta.json'), meta)
    return public_meta(meta)


def patch_app(app_id, body):
    with WRITE_LOCK:
        meta = load_meta(app_id)
        if not meta:
            raise HttpError(404, '没有这个小程序')
        for key in ('name', 'description'):
            if isinstance(body.get(key), str):
                meta[key] = body[key].strip()[:200]
        if isinstance(body.get('order'), int):
            meta['order'] = body['order']
        meta.update(clean_store_fields(body))
        write_json(os.path.join(app_dir(app_id), 'meta.json'), meta)
    return public_meta(meta)


def delete_app(app_id):
    d = app_dir(app_id)
    with WRITE_LOCK:
        if not os.path.isdir(d):
            raise HttpError(404, '没有这个小程序')
        shutil.rmtree(d)
