"""演示数据：电影（光影放映厅 movie-online 用）。

40 部虚构影片：前 16 部正在热映、中间 12 部即将上映、最后 12 部是经典老片（只出现在高分榜）。
片名 / 类型 / 配色是手写的（海报要好看、要像那么回事），演员、简介、短评按种子拼出来。
「今天」固定为 2026-10-26：上映日期、短评时间都相对它算，数据不随真实日期漂（截图基线可复现）。
"""
import datetime

from demo_common import avatar, fmt_count, img, nickname, person, rng, still

TODAY = datetime.date(2026, 10, 26)

# (片名, 英文名, 类型, 地区, 海报两色)
FILMS = [
    ('长河落日', 'The Long River', ['剧情', '历史'], '中国大陆', ('#1B2A6B', '#B64E6E')),
    ('星际回响', 'Echoes of Stars', ['科幻', '冒险'], '中国大陆', ('#2B1055', '#7A3A9A')),
    ('深海营救', 'Deep Rescue', ['动作', '灾难'], '中国大陆', ('#0D2B4A', '#2F7DBF')),
    ('雾都疑云', 'Foggy City', ['悬疑', '犯罪'], '中国香港', ('#232526', '#5B6467')),
    ('夏日寄语', 'Letters of Summer', ['爱情', '青春'], '中国台湾', ('#F7971E', '#FFD200')),
    ('山海之间', 'Between Mountains', ['纪录', '自然'], '中国大陆', ('#134E5E', '#71B280')),
    ('猫咪侦探社', 'Cat Detectives', ['动画', '喜剧'], '日本', ('#FF6E7F', '#BFE9FF')),
    ('最后一班地铁', 'The Last Train', ['剧情', '温情'], '中国大陆', ('#141E30', '#243B55')),
    ('无声告白', 'Silent Confession', ['剧情', '家庭'], '韩国', ('#606C88', '#3F4C6B')),
    ('疾速追击', 'Full Throttle', ['动作', '犯罪'], '美国', ('#C31432', '#240B36')),
    ('月光下的鲸', 'Whale Under Moon', ['奇幻', '冒险'], '中国大陆', ('#1A2980', '#26D0CE')),
    ('喜从天降', 'Lucky Day', ['喜剧'], '中国大陆', ('#F857A6', '#FF5858')),
    ('边境线', 'Borderline', ['战争', '动作'], '中国大陆', ('#3E5151', '#DECBA4')),
    ('时间的礼物', 'Gift of Time', ['科幻', '爱情'], '中国大陆', ('#4568DC', '#B06AB3')),
    ('老街烟火', 'Old Street', ['剧情', '喜剧'], '中国大陆', ('#B24592', '#F15F79')),
    ('冰原守望', 'Ice Watch', ['冒险', '剧情'], '加拿大', ('#83A4D4', '#B6FBFF')),
    ('流浪诗人', 'Wandering Poet', ['传记', '音乐'], '中国大陆', ('#3A1C71', '#D76D77')),
    ('暗夜行者', 'Night Walker', ['悬疑', '惊悚'], '美国', ('#000000', '#434343')),
    ('风起长安', 'Wind of Chang\'an', ['古装', '动作'], '中国大陆', ('#8E0E00', '#1F1C18')),
    ('小小厨神', 'Little Chef', ['动画', '家庭'], '中国大陆', ('#FDC830', '#F37335')),
    ('孤岛来信', 'Island Letters', ['剧情', '悬疑'], '中国大陆', ('#2C3E50', '#4CA1AF')),
    ('机械之心', 'Mechanical Heart', ['科幻', '动作'], '美国', ('#0F2027', '#2C5364')),
    ('春天的约定', 'Spring Promise', ['爱情', '剧情'], '日本', ('#FFAFBD', '#FFC3A0')),
    ('追光者', 'Light Chaser', ['运动', '励志'], '中国大陆', ('#F2994A', '#F2C94C')),
    ('迷城', 'Maze City', ['犯罪', '动作'], '中国香港', ('#16222A', '#3A6073')),
    ('龙门客栈新传', 'Dragon Inn Saga', ['武侠', '古装'], '中国大陆', ('#603813', '#B29F94')),
    ('银河邮差', 'Galaxy Postman', ['动画', '科幻'], '中国大陆', ('#5433FF', '#20BDFF')),
    ('外婆的厨房', 'Grandma\'s Kitchen', ['家庭', '温情'], '中国大陆', ('#ED8F03', '#FFB75E')),
    ('霸王别姬', 'Farewell My Concubine', ['剧情', '爱情'], '中国大陆', ('#7B4397', '#DC2430')),
    ('海上钢琴师', 'The Pianist at Sea', ['剧情', '音乐'], '意大利', ('#355C7D', '#C06C84')),
    ('千与千寻', 'Spirited Away', ['动画', '奇幻'], '日本', ('#11998E', '#38EF7D')),
    ('肖申克的救赎', 'The Shawshank', ['剧情', '犯罪'], '美国', ('#373B44', '#4286F4')),
    ('大话西游', 'A Chinese Odyssey', ['喜剧', '奇幻'], '中国香港', ('#F12711', '#F5AF19')),
    ('星际穿越', 'Interstellar', ['科幻', '冒险'], '美国', ('#000428', '#004E92')),
    ('活着', 'To Live', ['剧情', '历史'], '中国大陆', ('#5A3F37', '#2C7744')),
    ('天堂电影院', 'Cinema Paradiso', ['剧情', '爱情'], '意大利', ('#C04848', '#480048')),
    ('无间道', 'Infernal Affairs', ['犯罪', '悬疑'], '中国香港', ('#1D4350', '#A43931')),
    ('寻梦环游记', 'Coco', ['动画', '音乐'], '美国', ('#DA4453', '#89216B')),
    ('阿甘正传', 'Forrest Gump', ['剧情', '爱情'], '美国', ('#56AB2F', '#A8E063')),
    ('让子弹飞', 'Let the Bullets Fly', ['喜剧', '动作'], '中国大陆', ('#E65C00', '#F9D423')),
]
N_NOW, N_COMING = 16, 12

PLACES = ['江南小镇', '西北戈壁', '海滨渔村', '霓虹都市', '边陲哨所', '深山古寺', '雪原林场', '老城胡同']
JOBS = ['退休的老船长', '落魄的钢琴师', '刚入职的女警', '倔强的乡村教师', '失业的工程师', '年轻的急诊医生', '沉默的修表匠', '叛逆的高中生']
EVENTS = ['意外卷入一桩尘封二十年的旧案', '收到一封来自未来的信', '接下一个不可能完成的任务', '在一场暴雨中救下一个陌生孩子',
          '决定重返那座改变命运的城市', '发现父亲留下的一本密码日记', '被迫与宿敌组成临时搭档']
CONFLICTS = ['真相一层层剥开，每一个人都藏着秘密。', '时间只剩七天，所有退路都被切断。', '亲情、友情与信念被反复拉扯。',
             '一边是职责，一边是心底最柔软的牵挂。', '看似偶然的相遇，背后是一张精心编织的网。']
ENDINGS = ['这是一个关于告别与重逢的故事。', '影片用克制的镜头讲述普通人的勇气。', '笑中带泪，后劲十足。',
           '视听震撼，节奏紧凑，结尾的反转令人拍案。', '献给每一个在黑暗里仍然相信光的人。']
COMMENTS = ['看完久久不能平静，后劲太大了。', '演员演技在线，每个眼神都是戏。', '配乐太绝了，出了影院还在单曲循环。',
            '节奏有点慢，但最后半小时值回票价。', '特效很震撼，强烈建议 IMAX。', '剧情有几处硬伤，整体还算及格。',
            '哭成狗，带了一包纸巾不够用。', '带爸妈一起看的，全家都说好。', '摄影每一帧都能当壁纸。', '笑点密集，全场笑声不断。',
            '反转猜到了一半，另一半真没想到。', '期待了很久，没有让人失望。', '台词写得好，很多句子想摘抄。',
            '中段有点拖沓，可以剪掉十分钟。', '年度最佳候选，二刷预定。', '主角的成长线写得很扎实。', '比预告片好看太多。',
            '有些情节太煽情了，见仁见智吧。', '结尾留白恰到好处。', '小朋友看得很开心，大人也不无聊。']
ROLES = ['饰 林守望', '饰 苏晚', '饰 老秦', '饰 陆小川', '饰 船老大', '饰 秦嫂', '饰 顾队长', '饰 沈医生', '饰 江一鸣', '饰 白露']


def _summary(r, title):
    year = r.randint(1978, 2031)
    return (f'{year} 年，{r.choice(PLACES)}，{r.choice(JOBS)}{r.choice(EVENTS)}。{r.choice(CONFLICTS)}'
            f'当所有线索指向同一个名字，他终于明白，「{title}」从来不只是一个地名或一句誓言，而是一代人不肯放下的念想。'
            f'{r.choice(ENDINGS)}')


def _build():
    out = []
    for i, (title, en, genres, region, colors) in enumerate(FILMS):
        mid = i + 1
        r = rng('movie', mid)
        if i < N_NOW:
            status = 'now'
            release = TODAY - datetime.timedelta(days=r.randint(2, 60))
            score = round(r.uniform(7.2, 9.6), 1)
        elif i < N_NOW + N_COMING:
            status = 'coming'
            release = TODAY + datetime.timedelta(days=r.randint(5, 120))
            score = 0
        else:
            status = 'classic'
            release = datetime.date(r.randint(1988, 2015), r.randint(1, 12), r.randint(1, 28))
            score = round(r.uniform(9.0, 9.8), 1)
        cast = []
        for k in range(r.randint(5, 8)):
            cast.append({'id': f'p{mid}-{k}', 'name': person(r), 'role': ROLES[(mid + k) % len(ROLES)],
                         'avatar': avatar(mid * 3 + k)})
        out.append({
            'id': mid,
            'title': title,
            'enTitle': en,
            'genres': genres,
            'genreText': ' / '.join(genres),
            'region': region,
            'duration': r.randint(92, 168),
            'releaseDate': release.isoformat(),
            'status': status,
            'score': score,
            'scoreCount': r.randint(3000, 900000) if score else 0,
            'wishCount': r.randint(8000, 600000),
            'boxOffice': f'{r.uniform(0.3, 45):.1f}亿' if status != 'coming' else '',
            'director': {'name': person(r), 'avatar': avatar(mid * 7)},
            'cast': cast,
            'poster': img('poster', f'm{mid:02d}'),
            'colors': list(colors),
            'summary': _summary(r, title),
            'tags': (['IMAX'] if '科幻' in genres or '动作' in genres else []) + (['3D'] if r.random() < 0.3 else []) + (['中国巨幕'] if r.random() < 0.3 else []),
            'stills': [still(mid * 5 + k) for k in range(r.randint(6, 9))],
            'commentCount': r.randint(36, 88),
            # 电影原声歌单在 demo_music 里按片名生成，id = 100 + 电影 id
            'soundtrackPlaylistId': 100 + mid,
        })
    return out


MOVIES = _build()
BY_ID = {m['id']: m for m in MOVIES}

# 列表只给卡片要的字段（简介、剧照、短评都在详情里），一页 10 条的 JSON 才不臃肿
LIST_FIELDS = ('id', 'title', 'enTitle', 'genres', 'genreText', 'region', 'duration', 'releaseDate', 'status', 'score',
               'scoreCount', 'wishCount', 'boxOffice', 'poster', 'tags')


def _card(m):
    c = {k: m[k] for k in LIST_FIELDS}
    c['castText'] = ' / '.join(p['name'] for p in m['cast'][:3])
    c['wishText'] = fmt_count(m['wishCount'])
    return c


def list_movies(kind='now', q=''):
    q = (q or '').strip()
    if q:
        # 搜索不分类型：片名、英文名、导演、演员都算
        pool = [m for m in MOVIES if q in m['title'] or q.lower() in m['enTitle'].lower() or q in m['director']['name']
                or any(q in p['name'] for p in m['cast'])]
    elif kind == 'coming':
        pool = sorted((m for m in MOVIES if m['status'] == 'coming'), key=lambda m: m['releaseDate'])
    elif kind == 'top':
        pool = sorted((m for m in MOVIES if m['score']), key=lambda m: (-m['score'], m['id']))
    else:
        pool = [m for m in MOVIES if m['status'] == 'now']
    return [_card(m) for m in pool]


def comments(mid):
    m = BY_ID[mid]
    r = rng('comments', mid)
    out = []
    for i in range(m['commentCount']):
        base = m['score'] / 2 if m['score'] else 4
        star = max(1, min(5, round(r.gauss(base, 0.8))))
        days = i // 3 + r.randint(0, 2)
        text = r.choice(COMMENTS) + (r.choice(COMMENTS) if r.random() < 0.35 else '')
        out.append({
            'id': f'{mid}-{i + 1}',
            'user': {'name': nickname(r), 'avatar': avatar(mid + i * 5)},
            'star': star,
            'content': text,
            'date': (TODAY - datetime.timedelta(days=days)).isoformat(),
            'likes': int(r.paretovariate(1.6) * 6),
        })
    return out


def detail(mid):
    m = BY_ID.get(mid)
    if not m:
        return None
    d = dict(m)
    d['castText'] = ' / '.join(p['name'] for p in m['cast'][:3])
    d['wishText'] = fmt_count(m['wishCount'])
    d['hotComments'] = sorted(comments(mid), key=lambda c: -c['likes'])[:3]
    return d


def banners():
    picks = [MOVIES[1], MOVIES[0], MOVIES[N_NOW], MOVIES[10]]
    subs = ['IMAX 同步上映 · 观众评分 {s}', '国庆档口碑之作 · 累计 {b}', '{d} 上映 · 预售开启', '奇幻冒险 · 全家共赏']
    out = []
    for i, m in enumerate(picks):
        out.append({'id': f'mb{i + 1}', 'image': img('banner', f'movie{i + 1}'), 'title': m['title'],
                    'subtitle': subs[i].format(s=m['score'], b=m['boxOffice'], d=m['releaseDate'][5:]),
                    'colors': m['colors'],
                    'target': {'type': 'movie', 'id': m['id']}})
    return out


def genre_tags():
    seen = []
    for m in MOVIES:
        for g in m['genres']:
            if g not in seen:
                seen.append(g)
    return [{'name': g, 'count': sum(g in m['genres'] for m in MOVIES)} for g in seen]
