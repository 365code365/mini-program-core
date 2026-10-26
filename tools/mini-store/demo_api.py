"""演示数据接口 /api/demo/…：给示例小程序（光影放映厅、云听音乐、拾光图库、跳转中心）一个真实的后端。

公开、只读、带 `Access-Control-Allow-Origin: *`（H5 编译产物在别的域名下也要能请求）。
数据由 demo_movies / demo_music / demo_photos 按固定种子生成，同一请求永远同样结果；
图片与音频是 make_demo_assets.py 生成的真实文件，放在本文件旁边的 demo_assets/（部署后是
/opt/mini-store/app/demo_assets）。

接口（所有列表都支持 ?page=1&size=10，返回 {list, page, size, total, hasMore}；size 上限 50）：
    GET /api/demo/                              接口清单
    GET /api/demo/movies?type=now|coming|top&q=  电影列表（q 搜片名 / 英文名 / 导演 / 演员，给了 q 就不分类型）
    GET /api/demo/movies/<id>                   详情：stills 剧照、cast 演职员、hotComments、soundtrackPlaylistId
    GET /api/demo/movies/<id>/comments?sort=new|hot   短评分页
    GET /api/demo/banners?app=movie|music|gallery     首页轮播
    GET /api/demo/music/playlists?tag=          歌单（不给 tag 只列常规歌单；tag=影视原声 列电影原声）
    GET /api/demo/music/playlists/<id>          歌单详情 + songs
    GET /api/demo/music/songs?q=                歌曲列表 / 搜索
    GET /api/demo/music/songs/<id>              歌曲详情 + lrc（[mm:ss.xx] 文本）+ lyrics（[{time,text}]）+ audioUrl
    GET /api/demo/music/charts                  榜单（每个带前 20 首）
    GET /api/demo/photos?tag=&q=                图库（瀑布流：width/height 与 JPEG 实际尺寸一致）
    GET /api/demo/photos/<id>                   照片详情 + related
    GET /api/demo/tags?app=gallery|music|movie  标签 / 分类
    GET /api/demo/img/<kind>/<name>.jpg         图片：kind = poster still cover avatar photo banner
    GET /api/demo/audio/<n>.wav                 试听音频（单声道 16kHz 16bit）
返回里的图片 / 音频地址是绝对地址 http://<请求的 Host>/…，本机测试与服务器各自对。
"""
import hashlib
import json
import os
import re
import threading

import demo_movies
import demo_music
import demo_photos
from demo_common import absolutize, paginate

ASSETS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'demo_assets')
IMG_KINDS = {'poster', 'still', 'cover', 'avatar', 'photo', 'banner'}
NAME_RE = re.compile(r'^[a-z0-9_-]{1,40}\.(jpg|wav)$')
# Host 头原样拼进返回的地址：只认主机名 / IP / 端口的字符，别让人借它往 JSON 里塞别的东西
HOST_RE = re.compile(r'^[A-Za-z0-9.\-]{1,253}(:\d{1,5})?$|^\[[0-9A-Fa-f:]+\](:\d{1,5})?$')

_ETAGS = {}
_ETAG_LOCK = threading.Lock()


def _cors(h):
    h.send_header('Access-Control-Allow-Origin', '*')
    h.send_header('Access-Control-Allow-Methods', 'GET, OPTIONS')
    h.send_header('Access-Control-Allow-Headers', 'Content-Type, If-None-Match')


def _send_json(h, obj, status=200):
    data = json.dumps(obj, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
    h.send_response(status)
    h.send_header('Content-Type', 'application/json; charset=utf-8')
    h.send_header('Content-Length', str(len(data)))
    # 数据是确定的，可以让客户端缓存一会儿；但别太久，改了生成规则要能尽快看到
    h.send_header('Cache-Control', 'public, max-age=60' if status == 200 else 'no-store')
    _cors(h)
    h.end_headers()
    h.wfile.write(data)


def _not_found(h, msg='没有这个接口'):
    _send_json(h, {'error': msg}, 404)


def _etag(path):
    """按 (路径, 大小, 修改时间) 缓存内容摘要：文件不变就不重算，重新生成资源后自动换"""
    st = os.stat(path)
    key = (path, st.st_size, st.st_mtime_ns)
    with _ETAG_LOCK:
        tag = _ETAGS.get(key)
    if tag is None:
        with open(path, 'rb') as f:
            tag = '"' + hashlib.sha1(f.read()).hexdigest()[:20] + '"'
        with _ETAG_LOCK:
            _ETAGS[key] = tag
    return tag


def _send_file(h, sub, name, ctype):
    """sub 只能是白名单里的目录名，name 只能是 [a-z0-9_-] + 扩展名；最后再用 realpath 兜一次底（防穿越）"""
    if not NAME_RE.match(name):
        return _not_found(h, '没有这个文件')
    root = os.path.realpath(ASSETS)
    path = os.path.realpath(os.path.join(root, sub, name))
    if not path.startswith(root + os.sep) or not os.path.isfile(path):
        return _not_found(h, '没有这个文件')
    etag = _etag(path)
    if h.headers.get('If-None-Match') == etag:
        h.send_response(304)
        h.send_header('ETag', etag)
        _cors(h)
        h.end_headers()
        return
    size = os.path.getsize(path)
    h.send_response(200)
    h.send_header('Content-Type', ctype)
    h.send_header('Content-Length', str(size))
    h.send_header('Cache-Control', 'public, max-age=86400')
    h.send_header('ETag', etag)
    _cors(h)
    h.end_headers()
    with open(path, 'rb') as f:
        while True:
            chunk = f.read(256 << 10)
            if not chunk:
                break
            h.wfile.write(chunk)


def _int(s):
    return int(s) if s.isdigit() and len(s) < 8 else None


def _base(h):
    host = (h.headers.get('Host') or '').strip()
    return 'http://' + (host if HOST_RE.match(host) else '127.0.0.1')


def _banners(app):
    return {'movie': demo_movies.banners, 'music': demo_music.banners, 'gallery': demo_photos.banners}.get(app, demo_movies.banners)()


def _tags(app):
    if app == 'music':
        return demo_music.tags()
    if app == 'movie':
        return demo_movies.genre_tags()
    return demo_photos.tags()


def _route(p, q):
    """返回 (数据, 状态码)；None 表示没有这个接口"""
    n = len(p)
    if n == 0:
        return {'endpoints': [line.strip() for line in __doc__.splitlines() if line.strip().startswith('GET ')]}
    if p[0] == 'movies':
        if n == 1:
            return paginate(demo_movies.list_movies(q.get('type', 'now'), q.get('q', '')), q)
        mid = _int(p[1])
        if mid is None or mid not in demo_movies.BY_ID:
            return None
        if n == 2:
            return demo_movies.detail(mid)
        if n == 3 and p[2] == 'comments':
            cs = demo_movies.comments(mid)
            if q.get('sort') == 'hot':
                cs = sorted(cs, key=lambda c: -c['likes'])
            return paginate(cs, q)
        return None
    if p == ['banners']:
        return {'list': _banners(q.get('app', 'movie'))}
    if p == ['tags']:
        return {'list': _tags(q.get('app', 'gallery'))}
    if p[0] == 'music' and n >= 2:
        if p[1:] == ['playlists']:
            return paginate(demo_music.list_playlists(q.get('tag', '')), q)
        if p[1:] == ['charts']:
            return {'list': demo_music.charts()}
        if p[1:] == ['songs']:
            kw = q.get('q', '').strip()
            songs = [s for s in demo_music.SONG_LIST if not kw or kw in s['name'] or kw in s['singer']['name']]
            return paginate(songs, q, 20)
        if n == 3 and p[1] in ('playlists', 'songs'):
            i = _int(p[2])
            if i is None:
                return None
            return demo_music.playlist_detail(i) if p[1] == 'playlists' else demo_music.song_detail(i)
        return None
    if p[0] == 'photos':
        if n == 1:
            return paginate(demo_photos.list_photos(q.get('tag', ''), q.get('q', '').strip()), q, 20)
        if n == 2 and _int(p[1]) is not None:
            return demo_photos.detail(_int(p[1]))
    return None


def handle(h, method, p, q):
    """server.py 的 route() 把 /api/demo/ 之后的路径段（p）与查询参数（q）交过来"""
    if method == 'OPTIONS':
        h.send_response(204)
        _cors(h)
        h.send_header('Access-Control-Max-Age', '86400')
        h.send_header('Content-Length', '0')
        h.end_headers()
        return
    if method != 'GET':
        return _send_json(h, {'error': '演示接口只读'}, 405)
    if len(p) == 3 and p[0] == 'img' and p[1] in IMG_KINDS and p[2].endswith('.jpg'):
        return _send_file(h, p[1], p[2], 'image/jpeg')
    if len(p) == 2 and p[0] == 'audio' and p[1].endswith('.wav'):
        return _send_file(h, 'audio', p[1], 'audio/wav')
    if p and p[0] in ('img', 'audio'):
        return _not_found(h, '没有这个文件')
    data = _route(p, q)
    if data is None:
        return _not_found(h)
    _send_json(h, absolutize(data, _base(h)))
