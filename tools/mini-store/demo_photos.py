"""演示数据：图库（拾光图库 photo-wall 用）。

60 张照片，宽固定 600、高 400~900 不等（瀑布流要的就是高矮不齐）。
每张的「画面」由主标签决定（风光画山、城市画楼、极简画几何……），make_demo_assets.py 按 photo_specs() 画，
所以列表里给的 width / height 与真实 JPEG 的尺寸严格一致：小程序按宽高比排双列，不用等图片下载完。
"""
import datetime

from demo_common import avatar, img, nickname, rng

TAGS = ['风光', '城市', '人像', '美食', '动物', '建筑', '植物', '极简', '夜景', '旅行']
# 每个标签的配色（渐变两端）
TAG_COLORS = {
    '风光': ('#355C7D', '#F8B195'), '城市': ('#2C3E50', '#FD746C'), '人像': ('#ee9ca7', '#ffdde1'),
    '美食': ('#F09819', '#EDDE5D'), '动物': ('#56AB2F', '#A8E063'), '建筑': ('#757F9A', '#D7DDE8'),
    '植物': ('#134E5E', '#71B280'), '极简': ('#E0EAFC', '#CFDEF3'), '夜景': ('#0F0C29', '#302B63'),
    '旅行': ('#00B4DB', '#0083B0'),
}
TITLES = {
    '风光': ['雪山日出', '云海之上', '秋日湖畔', '草原落日', '峡谷晨雾', '海岸线'],
    '城市': ['下班高峰', '雨后街角', '天际线', '老城新貌', '十字路口', '高架桥'],
    '人像': ['逆光少女', '街头的笑', '窗边', '午后时光', '奔跑吧', '回眸'],
    '美食': ['周末早午餐', '深夜食堂', '一碗热汤面', '水果拼盘', '手冲咖啡', '烤肉滋滋'],
    '动物': ['橘猫晒太阳', '柴犬微笑', '林间小鹿', '海鸥', '熊猫吃竹子', '松鼠'],
    '建筑': ['几何线条', '旋转楼梯', '红墙黄瓦', '玻璃幕墙', '老教堂', '拱门'],
    '植物': ['春樱', '雨后蕨类', '向日葵田', '仙人掌', '银杏大道', '荷塘月色'],
    '极简': ['一个人', '留白', '线与面', '蓝色时刻', '影子', '孤独的树'],
    '夜景': ['万家灯火', '星轨', '霓虹', '夜市', '月亮升起', '江边夜色'],
    '旅行': ['在路上', '列车窗外', '小镇时光', '背包客', '海岛假日', '古镇'],
}
CAMERAS = ['Sony A7M4', 'Canon R6', 'Nikon Z6II', 'Fujifilm X-T5', 'iPhone 16 Pro', 'Leica Q3', 'DJI Mavic 3']
TODAY = datetime.date(2026, 10, 26)
N_PHOTOS = 60


def _build():
    out = []
    for i in range(N_PHOTOS):
        pid = i + 1
        r = rng('photo', pid)
        tag = TAGS[i % len(TAGS)]
        extra = r.sample([t for t in TAGS if t != tag], r.randint(1, 2))
        h = r.randrange(400, 901, 10)
        title = TITLES[tag][(i // len(TAGS)) % len(TITLES[tag])]
        likes = int(r.paretovariate(1.3) * 120)
        out.append({
            'id': pid, 'title': title, 'url': img('photo', f'p{pid:02d}'), 'width': 600, 'height': h,
            'ratio': round(h / 600, 4), 'color': TAG_COLORS[tag][0], 'tags': [tag] + extra,
            'author': {'name': nickname(r), 'avatar': avatar(pid * 11)},
            'likes': likes, 'views': likes * r.randint(8, 30), 'downloads': likes // r.randint(2, 6),
            'camera': r.choice(CAMERAS),
            'exif': f'f/{r.choice(["1.8", "2.8", "4", "5.6", "8"])} · 1/{r.choice([60, 125, 250, 500, 1000])}s · ISO {r.choice([100, 200, 400, 800, 1600])}',
            'location': r.choice(['云南 · 大理', '浙江 · 杭州', '四川 · 稻城', '上海', '北京', '新疆 · 喀纳斯', '福建 · 厦门', '西藏 · 林芝']),
            'description': f'{title}。{r.choice(["那天的光线刚刚好。", "按下快门的瞬间，风停了。", "等了两个小时才等到这一刻。", "随手一拍，意外地喜欢。", "希望你也能感受到这份安静。"])}',
            'createdAt': (TODAY - datetime.timedelta(days=i * 2 + r.randint(0, 1))).isoformat(),
        })
    return out


PHOTOS = _build()
BY_ID = {p['id']: p for p in PHOTOS}


def photo_specs():
    """make_demo_assets.py 用：(文件名, 宽, 高, 主标签, 标题, 两色)"""
    return [(f'p{p["id"]:02d}', p['width'], p['height'], p['tags'][0], p['title'], TAG_COLORS[p['tags'][0]]) for p in PHOTOS]


def list_photos(tag='', q=''):
    out = PHOTOS
    if tag:
        out = [p for p in out if tag in p['tags']]
    if q:
        out = [p for p in out if q in p['title'] or q in p['description'] or q in p['author']['name']]
    return list(out)


def detail(pid):
    p = BY_ID.get(pid)
    if not p:
        return None
    # 相关推荐：同主标签的、其次是共享任一标签的，去掉自己，最多 6 张
    same = [x for x in PHOTOS if x['id'] != pid and x['tags'][0] == p['tags'][0]]
    near = [x for x in PHOTOS if x['id'] != pid and x not in same and set(x['tags']) & set(p['tags'])]
    return dict(p, related=(same + near)[:6])


def tags():
    return [{'name': t, 'count': len(list_photos(t)), 'color': TAG_COLORS[t][0], 'cover': next(p['url'] for p in PHOTOS if p['tags'][0] == t)}
            for t in TAGS]


def banners():
    picks = [('本周精选', '秋日里的金色时光', '风光'), ('夜景摄影大赛', '投稿赢取万元大奖', '夜景'), ('萌宠日记', '治愈你的每一天', '动物')]
    return [{'id': f'gb{i + 1}', 'image': img('banner', f'gallery{i + 1}'), 'title': t, 'subtitle': s,
             'colors': list(TAG_COLORS[tag]), 'target': {'type': 'tag', 'tag': tag}} for i, (t, s, tag) in enumerate(picks)]
