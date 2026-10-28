"""mini-store 的接口测试：起一个临时数据目录 + 随机端口的真服务器，用 HTTP 打。

    python3 -m unittest tools/mini-store/test_store.py

为什么走真 HTTP 而不是直接调函数：分页参数的解析、「不给分页参数 = 全量」的兼容、
Host 头拼出的绝对地址、图片的 ETag / 防穿越，都发生在 Handler 里，绕过它就测不到。
图片 / 音频的用例要先跑过 make_demo_assets.py（demo_assets/ 不存在就跳过，并说明原因）。
"""
import io
import json
import os
import shutil
import sys
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
_TMP = tempfile.mkdtemp(prefix='mini-store-test-')
# store_pkg 在 import 时就读 MINI_STORE_DATA：必须先设环境变量再 import
os.environ['MINI_STORE_DATA'] = _TMP
sys.path.insert(0, HERE)

import demo_api  # noqa: E402
import server  # noqa: E402
import store_pkg  # noqa: E402

N_APPS = 23


def _zip(name):
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w') as z:
        z.writestr('app.json', json.dumps({'pages': ['pages/index/index'], 'window': {'navigationBarTitleText': name}}))
        z.writestr('pages/index/index.wxml', '<view>hi</view>')
    return buf.getvalue()


def _json(d, name):
    with open(os.path.join(d, name), encoding='utf-8') as f:
        return json.load(f)


class StoreTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        os.makedirs(os.path.join(_TMP, 'apps'), exist_ok=True)
        for i in range(N_APPS):
            cat = '娱乐' if i % 3 == 0 else '工具'
            store_pkg.save_app(f'app-{i:02d}', _zip(f'应用{i}'), f'测试应用{i:02d}', f'第 {i} 个', None,
                               {'category': cat, 'tags': '演示,测试'})
        server.Handler.cfg = {'username': 'admin', 'salt': '00', 'rounds': 1, 'hash': '', 'secret': '00' * 32}
        cls.httpd = server.ThreadingHTTPServer(('127.0.0.1', 0), server.Handler)
        cls.httpd.daemon_threads = True
        server.Handler.log_message = lambda *a: None
        cls.port = cls.httpd.server_address[1]
        threading.Thread(target=cls.httpd.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls):
        cls.httpd.shutdown()
        cls.httpd.server_close()
        shutil.rmtree(_TMP, ignore_errors=True)

    def req(self, path, headers=None):
        r = urllib.request.Request(f'http://127.0.0.1:{self.port}{path}', headers=headers or {})
        try:
            with urllib.request.urlopen(r, timeout=10) as res:
                return res.status, dict(res.headers), res.read()
        except urllib.error.HTTPError as e:
            return e.code, dict(e.headers), e.read()

    def get_json(self, path):
        status, headers, body = self.req(path)
        self.assertEqual(status, 200, body[:200])
        return json.loads(body), headers

    # ── 任务 B：/api/apps 分页 ──
    def test_apps_full_list_compat(self):
        data, _ = self.get_json('/api/apps')
        self.assertEqual(len(data['apps']), N_APPS)
        self.assertNotIn('page', data)          # 不给分页参数：形状与从前完全一样（App 依赖全量）

    def test_apps_paging(self):
        p1, _ = self.get_json('/api/apps?page=1&pageSize=10')
        p3, _ = self.get_json('/api/apps?page=3&pageSize=10')
        self.assertEqual((p1['total'], p1['page'], p1['pageSize'], len(p1['apps'])), (N_APPS, 1, 10, 10))
        self.assertEqual(len(p3['apps']), N_APPS - 20)
        full, _ = self.get_json('/api/apps')
        ids = [a['id'] for a in full['apps']]
        self.assertEqual([a['id'] for a in p1['apps']], ids[:10])
        only_size, _ = self.get_json('/api/apps?pageSize=5')
        self.assertEqual((only_size['page'], len(only_size['apps'])), (1, 5))
        bad, _ = self.get_json('/api/apps?page=abc&pageSize=99999')
        self.assertEqual((bad['page'], bad['pageSize']), (1, 100))

    def test_search_paging(self):
        cat_total = sum(1 for i in range(N_APPS) if i % 3 == 0)
        a, _ = self.get_json('/api/apps?category=%E5%A8%B1%E4%B9%90&page=1&pageSize=3')
        b, _ = self.get_json('/api/apps?category=%E5%A8%B1%E4%B9%90&page=2&pageSize=3')
        self.assertEqual(a['total'], cat_total)
        self.assertEqual(len(a['apps']), 3)
        self.assertFalse({x['id'] for x in a['apps']} & {x['id'] for x in b['apps']})
        q, _ = self.get_json('/api/apps?q=%E6%B5%8B%E8%AF%95&page=1&pageSize=4')
        self.assertEqual((q['total'], len(q['apps'])), (N_APPS, 4))
        legacy, _ = self.get_json('/api/apps?q=%E6%B5%8B%E8%AF%95&limit=200')
        self.assertEqual(len(legacy['apps']), N_APPS)

    # ── 小游戏：sample-game/play-* 的真包上架，标成 kind=game；默认列表不给（老客户端只会开 app.json） ──
    def test_game_packages(self):
        root = os.path.join(os.path.dirname(os.path.dirname(HERE)), 'sample-game')
        games = sorted(d for d in os.listdir(root) if d.startswith('play-')) if os.path.isdir(root) else []
        if not games:
            self.skipTest('没有 sample-game/play-*')
        for g in games:
            d = os.path.join(root, g)
            meta, proj = _json(d, 'store.meta.json'), _json(d, 'project.config.json')
            self.assertEqual(proj['compileType'], 'game', g)
            buf = io.BytesIO()
            with zipfile.ZipFile(buf, 'w') as z:            # 包一层目录：Finder「压缩」出来的布局
                for base, _, files in os.walk(d):
                    for f in files:
                        z.write(os.path.join(base, f), os.path.join(g, os.path.relpath(os.path.join(base, f), d)))
            m = store_pkg.save_app(g, buf.getvalue(), meta['name'], meta['description'], None,
                                   {'category': meta['category'], 'tags': ','.join(meta['tags'])})
            self.assertEqual((m['kind'], m['kindLabel'], m['appId'], m['category'], m['pages']),
                             ('game', '小游戏', proj['appid'], '游戏', 0), g)
            self.assertTrue(m['hasIcon'], g)
            ori = _json(d, 'game.json').get('deviceOrientation', 'portrait')
            self.assertEqual(m['orientation'], ori, g)
            with zipfile.ZipFile(os.path.join(_TMP, 'apps', g, 'package.zip')) as z:
                self.assertIn('game.js', z.namelist())
                self.assertEqual(json.loads(z.read('project.config.json'))['compileType'], 'game')
        default, _ = self.get_json('/api/apps')
        self.assertFalse([a for a in default['apps'] if a['kind'] == 'game'])
        self.assertTrue(all(a['kindLabel'] == '小程序' for a in default['apps']))
        only, _ = self.get_json('/api/apps?kind=game')
        self.assertEqual(sorted(a['id'] for a in only['apps']), games)
        both, _ = self.get_json('/api/apps?kind=all&page=1&pageSize=100')
        self.assertEqual(both['total'], N_APPS + len(games))
        found, _ = self.get_json('/api/apps?kind=game&q=%E8%B4%AA%E5%90%83%E8%9B%87')    # 贪吃蛇
        self.assertEqual([a['id'] for a in found['apps']], ['play-snake'])
        cats, _ = self.get_json('/api/categories')
        self.assertNotIn('游戏', [c['name'] for c in cats['categories']])
        cats, _ = self.get_json('/api/categories?kind=all')
        self.assertIn({'name': '游戏', 'count': len(games)}, cats['categories'])
        one, _ = self.get_json(f'/api/apps/{games[0]}')         # 按 id 取单个不看 kind：打开 / 升级检查照常
        self.assertEqual(one['kind'], 'game')
        self.assertEqual(self.req('/api/apps?kind=nope')[0], 400)

    def test_game_package_rejects(self):
        def z(files):
            buf = io.BytesIO()
            with zipfile.ZipFile(buf, 'w') as w:
                for k, v in files.items():
                    w.writestr(k, v)
            return buf.getvalue()
        cases = {
            'game.js': {'game.json': '{}'},
            'deviceOrientation': {'game.json': '{"deviceOrientation":"sideways"}', 'game.js': ''},
            '同时有': {'game.json': '{}', 'game.js': '', 'app.json': '{"pages":["a"]}'},
        }
        for want, files in cases.items():
            with self.assertRaises(store_pkg.HttpError) as cm:
                store_pkg.save_app('bad-game', z(files), 'x', '', None)
            self.assertIn(want, cm.exception.message)
        # 同一个 id 不许在小程序 / 小游戏之间换
        with self.assertRaises(store_pkg.HttpError) as cm:
            store_pkg.save_app('app-00', z({'game.json': '{}', 'game.js': ''}), 'x', '', None)
        self.assertEqual(cm.exception.status, 409)
        self.assertIsNone(store_pkg.load_meta('bad-game'))

    # ── 任务 A：演示接口 ──
    def test_demo_movies_paging(self):
        p1, h = self.get_json('/api/demo/movies?type=now&page=1&size=10')
        self.assertEqual(h.get('Access-Control-Allow-Origin'), '*')
        self.assertEqual((p1['page'], p1['size'], len(p1['list'])), (1, 10, 10))
        self.assertTrue(p1['hasMore'])
        p2, _ = self.get_json(f'/api/demo/movies?type=now&page=2&size=10')
        self.assertEqual(len(p2['list']), p1['total'] - 10)
        self.assertFalse(p2['hasMore'])
        top, _ = self.get_json('/api/demo/movies?type=top&size=50')
        scores = [m['score'] for m in top['list']]
        self.assertEqual(scores, sorted(scores, reverse=True))
        # 同一请求永远同样结果
        again, _ = self.get_json('/api/demo/movies?type=now&page=1&size=10')
        self.assertEqual(again, p1)

    def test_demo_movie_search(self):
        r, _ = self.get_json('/api/demo/movies?q=%E6%98%9F%E9%99%85')   # 星际
        self.assertTrue(r['total'] >= 1)
        self.assertTrue(all('星际' in m['title'] or '星际' in m['castText'] for m in r['list']))

    def test_demo_movie_detail(self):
        d, _ = self.get_json('/api/demo/movies/1')
        for k in ('title', 'score', 'summary', 'stills', 'cast', 'poster', 'releaseDate', 'duration', 'genres',
                  'hotComments', 'soundtrackPlaylistId', 'director'):
            self.assertIn(k, d)
        self.assertGreaterEqual(len(d['stills']), 6)
        self.assertTrue(d['poster'].startswith(f'http://127.0.0.1:{self.port}/api/demo/img/poster/'))
        self.assertTrue(d['cast'][0]['avatar'].startswith('http://'))
        c, _ = self.get_json('/api/demo/movies/1/comments?page=1&size=5')
        self.assertEqual((len(c['list']), c['total']), (5, d['commentCount']))
        self.assertEqual(self.req('/api/demo/movies/999')[0], 404)
        self.assertEqual(self.req('/api/demo/movies/abc')[0], 404)

    def test_demo_music_and_photos(self):
        pl, _ = self.get_json('/api/demo/music/playlists?page=1&size=6')
        self.assertEqual(len(pl['list']), 6)
        self.assertTrue(pl['hasMore'])
        ost, _ = self.get_json('/api/demo/music/playlists/101')
        self.assertIn('电影原声', ost['name'])
        self.assertEqual(len(ost['songs']), ost['songCount'])
        song, _ = self.get_json('/api/demo/music/songs/1')
        self.assertTrue(song['audioUrl'].endswith('.wav'))
        self.assertTrue(song['lrc'].startswith('[00:00.00]'))
        times = [x['time'] for x in song['lyrics']]
        self.assertEqual(times, sorted(times))
        self.assertLess(times[-1], song['trialSeconds'])
        charts, _ = self.get_json('/api/demo/music/charts')
        self.assertEqual(len(charts['list']), 4)
        ph, _ = self.get_json('/api/demo/photos?tag=%E9%A3%8E%E5%85%89&page=1&size=4')
        self.assertTrue(all('风光' in p['tags'] for p in ph['list']))
        self.assertEqual(len({p['height'] for p in self.get_json('/api/demo/photos?size=20')[0]['list']}) > 3, True)
        one, _ = self.get_json('/api/demo/photos/3')
        self.assertTrue(1 <= len(one['related']) <= 6)
        for app in ('movie', 'music', 'gallery'):
            self.assertTrue(self.get_json(f'/api/demo/banners?app={app}')[0]['list'])
        self.assertTrue(self.get_json('/api/demo/tags?app=gallery')[0]['list'])
        self.assertEqual(self.req('/api/demo/nope')[0], 404)

    @unittest.skipUnless(os.path.isdir(demo_api.ASSETS), '没有 demo_assets/：先跑 python3 tools/mini-store/make_demo_assets.py')
    def test_demo_files(self):
        status, h, body = self.req('/api/demo/img/poster/m01.jpg')
        self.assertEqual(status, 200)
        self.assertEqual(h['Content-Type'], 'image/jpeg')
        self.assertTrue(body.startswith(b'\xff\xd8'))
        self.assertIn('max-age=86400', h['Cache-Control'])
        status304, _, _ = self.req('/api/demo/img/poster/m01.jpg', {'If-None-Match': h['ETag']})
        self.assertEqual(status304, 304)
        status, h, body = self.req('/api/demo/audio/1.wav')
        self.assertEqual((status, h['Content-Type'], body[:4]), (200, 'audio/wav', b'RIFF'))
        # 防穿越：.. 段、编码过的 ..、白名单外的目录、奇怪的文件名，一律 404
        for bad in ('/api/demo/img/../server.py', '/api/demo/img/poster/..%2F..%2Fserver.py', '/api/demo/img/poster/%2e%2e.jpg',
                    '/api/demo/img/audio/1.wav', '/api/demo/img/x/m01.jpg', '/api/demo/audio/../../server.py',
                    '/api/demo/img/poster/M01.jpg', '/api/demo/img/poster/nope.jpg'):
            self.assertEqual(self.req(bad)[0], 404, bad)

    def test_demo_photo_size_matches_file(self):
        if not os.path.isdir(demo_api.ASSETS):
            self.skipTest('没有 demo_assets/')
        try:
            from PIL import Image
        except ImportError:
            self.skipTest('没有 Pillow')
        for p in self.get_json('/api/demo/photos?size=50')[0]['list'][:10]:
            name = p['url'].rsplit('/', 1)[1]
            with Image.open(os.path.join(demo_api.ASSETS, 'photo', name)) as im:
                self.assertEqual(im.size, (p['width'], p['height']))


if __name__ == '__main__':
    unittest.main()
