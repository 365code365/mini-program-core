"""演示数据：音乐（云听音乐 music-online 用）。

60 首歌 / 20 张专辑 / 24 个歌单 / 4 个榜单，另有每部电影一个「电影原声」歌单（id = 100 + 电影 id，
封面直接用那部电影的海报），光影放映厅的「听原声」跳过来打开的就是它。
音频只有 N_AUDIO 段真实的 wav（make_demo_assets.py 合成），按歌曲 id 轮流复用；
所以 duration 是「整首歌」的展示时长，trialSeconds 才是能播的长度，歌词时间轴也排在试听段内 ——
否则播放页的歌词高亮会停在第三行不动。
"""
from demo_common import AUDIO_SECONDS, audio, avatar, cover, fmt_count, img, nickname, rng
from demo_movies import MOVIES

SINGERS = ['林夕语', '周一然', '陈星野', '苏小暖', '南风乐队', '许清和', '顾北', '白日梦想家', '江枫', '叶知秋',
           '程亦', '唐小糖', '海盐汽水', '温言', '陆离', '秦川', '夏未央', '梁上君', '鹿鸣', '回声计划']
SONGS = ['晚风告白', '海边的信', '第七个夏天', '城南旧事', '星河滚烫', '慢慢喜欢你', '云边小卖部', '一万次心动', '旧时光',
         '下雨的周末', '月亮邮局', '橘子汽水', '远方的你', '风吹麦浪', '凌晨三点', '纸飞机', '山茶花开', '漫步人间',
         '夜航船', '小幸运', '春日来信', '十字路口', '白鸽', '平凡之路', '时光机', '倒数', '落日飞行', '你的样子',
         '雪落下的声音', '光年之外', '青柠', '晴天以后', '梦里水乡', '少年游', '南方', '北岛', '人间烟火', '天空之城',
         '初见', '蝉鸣', '夏夜晚风', '盛夏光年', '海底', '追梦人', '明天会更好', '不说再见', '长安', '江湖', '归途',
         '慢车', '雨巷', '萤火', '灯塔', '鲸落', '晚安', '日出', '候鸟', '岛屿', '微光', '星空下']
ALBUMS = ['夏日限定', '城市漫游', '晚风集', '时光的礼物', '远行', '一个人的旅行', '安静', '星辰大海', '旧梦', '小宇宙',
          '向阳而生', '雨季', '南方以南', '海风', '纸短情长', '慢生活', '未完待续', '回声', '光', '浮生']
LYRIC_LINES = ['风从海上来 吹乱了谁的发', '我站在路口 等一盏灯亮起', '那年夏天的蝉鸣 还在耳边', '你说过的话 我都记得',
               '城市的灯火 照不亮回家的路', '把思念折成纸飞机', '飞过山 飞过海 飞到你窗台', '月亮挂在天边 像你的眼',
               '慢慢走 别回头', '时间是一条河 带走了我们', '雨落在屋檐 滴答滴答', '我们都是追光的人',
               '如果明天下雨 就在家等你', '星星睡了 我还醒着', '晚风轻轻吹过 心事都变成歌', '这一路有你 就不怕远',
               '褪色的照片里 你笑得那么甜', '我把夏天藏进口袋', '黑夜给了我黑色的眼睛', '人海里 一眼就认出你']
PLAYLIST_TAGS = ['华语', '流行', '民谣', '摇滚', '电子', '轻音乐', '古风', '治愈', '运动', '夜晚', '学习', '影视原声']
PLAYLIST_NAMES = ['深夜书房 · 安静陪你读完一本书', '通勤路上的好心情', '华语经典 · 那些年我们单曲循环的歌', '民谣与远方',
                  '燃脂跑步 BPM160', '周末慵懒午后', '下雨天听的歌', '古风 · 一曲江湖梦', '电子迷幻夜', '治愈系 · 抱抱你',
                  '摇滚不死', '专注学习 · 白噪音与钢琴', '车载必备 · 一路向西', '失眠的时候听什么', '夏日海边 · 冰汽水',
                  '小众宝藏歌手合集', '咖啡馆里的爵士', '一个人的 KTV', '温柔女声', '少年感男声', '纯音乐 · 放空大脑',
                  '回忆杀 · 2000 年代', '早安 · 元气满满', '晚安 · 好梦']
# 歌单封面的配色（c21~c30 循环用；c01~c20 是专辑封面）
COVER_COLORS = ['#ff9a9e', '#a18cd1', '#fbc2eb', '#84fab0', '#8fd3f4', '#fccb90', '#e0c3fc', '#f6d365', '#96e6a1', '#4facfe',
                '#43e97b', '#fa709a', '#30cfd0', '#a8edea', '#5ee7df', '#d299c2', '#fddb92', '#89f7fe', '#c471f5', '#f093fb']
COVER_COLORS2 = ['#fecfef', '#fbc2eb', '#a6c1ee', '#8fd3f4', '#84fab0', '#d57eeb', '#8ec5fc', '#fda085', '#d4fc79', '#00f2fe',
                 '#38f9d7', '#fee140', '#330867', '#fed6e3', '#b490ca', '#fef9d7', '#d1fdff', '#66a6ff', '#fa71cd', '#f5576c']


def _fmt_dur(s):
    return f'{s // 60}:{s % 60:02d}'


def cover_specs():
    """30 张封面的 (文字, 颜色1, 颜色2)：make_demo_assets.py 照这个画"""
    out = []
    for i in range(30):
        text = ALBUMS[i] if i < len(ALBUMS) else PLAYLIST_TAGS[i - len(ALBUMS)]
        out.append((text, COVER_COLORS[i % 20], COVER_COLORS2[(i * 7) % 20]))
    return out


def _lrc(r, name, singer):
    """[mm:ss.xx] 一行一句，排在试听段（AUDIO_SECONDS 秒）之内"""
    lines = [(0.0, f'{name} - {singer}')]
    t = 2.0
    picks = r.sample(LYRIC_LINES, 10)
    for text in picks:
        if t > AUDIO_SECONDS - 1:
            break
        lines.append((round(t, 2), text))
        t += r.uniform(1.8, 2.6)
    lrc = '\n'.join(f'[{int(s // 60):02d}:{s % 60:05.2f}]{x}' for s, x in lines)
    return lrc, [{'time': s, 'text': x} for s, x in lines]


def _build_songs():
    out = []
    for i, name in enumerate(SONGS):
        sid = i + 1
        r = rng('song', sid)
        si, ai = r.randrange(len(SINGERS)), r.randrange(len(ALBUMS))
        dur = r.randint(168, 312)
        out.append({'id': sid, 'name': name, 'singer': {'id': si + 1, 'name': SINGERS[si], 'avatar': avatar(si)},
                    'album': {'id': ai + 1, 'name': ALBUMS[ai]}, 'cover': cover(ai), 'duration': dur,
                    'durationText': _fmt_dur(dur), 'trialSeconds': AUDIO_SECONDS, 'audioUrl': audio(sid),
                    'vip': r.random() < 0.18, 'hot': r.random() < 0.25, 'plays': r.randint(20000, 9000000)})
    return out


SONG_LIST = _build_songs()
SONG_BY_ID = {s['id']: s for s in SONG_LIST}


def _playlist(pid, name, tags, cov, r, desc):
    songs = r.sample(range(1, len(SONG_LIST) + 1), r.randint(12, 22))
    play = r.randint(12000, 48000000)
    return {'id': pid, 'name': name, 'cover': cov, 'tags': tags, 'description': desc,
            'creator': {'name': nickname(r), 'avatar': avatar(pid)}, 'playCount': play, 'playCountText': fmt_count(play),
            'songIds': songs, 'songCount': len(songs), 'subscribed': r.randint(300, 200000)}


def _build_playlists():
    out = []
    for i, name in enumerate(PLAYLIST_NAMES):
        r = rng('playlist', i + 1)
        tags = r.sample(PLAYLIST_TAGS[:-1], 2)
        out.append(_playlist(i + 1, name, tags, cover(20 + i % 10), r,
                             f'精选 {"、".join(tags)} 好歌，{r.choice(["适合一个人慢慢听", "陪你度过每个日落", "打开就停不下来", "每周更新"])}。'))
    for m in MOVIES:
        r = rng('ost', m['id'])
        out.append(_playlist(100 + m['id'], f'《{m["title"]}》电影原声', ['影视原声', m['genres'][0]], m['poster'], r,
                             f'电影《{m["title"]}》（{m["enTitle"]}）的原声配乐与主题曲。'))
    return out


PLAYLISTS = _build_playlists()
PL_BY_ID = {p['id']: p for p in PLAYLISTS}


def _card(p):
    return {k: v for k, v in p.items() if k != 'songIds'}


def list_playlists(tag=''):
    if tag:
        return [_card(p) for p in PLAYLISTS if tag in p['tags']]
    # 不按标签筛时只给常规歌单（40 个原声歌单会把首页挤满）
    return [_card(p) for p in PLAYLISTS if p['id'] < 100]


def playlist_detail(pid):
    p = PL_BY_ID.get(pid)
    if not p:
        return None
    return dict(_card(p), songs=[SONG_BY_ID[s] for s in p['songIds']])


def song_detail(sid):
    s = SONG_BY_ID.get(sid)
    if not s:
        return None
    lrc, lines = _lrc(rng('lrc', sid), s['name'], s['singer']['name'])
    return dict(s, lrc=lrc, lyrics=lines, composer=s['singer']['name'], lyricist=SINGERS[(sid * 3) % len(SINGERS)])


def charts():
    out = []
    for i, (name, color) in enumerate([('飙升榜', '#ff5a5f'), ('新歌榜', '#1fb6ff'), ('热歌榜', '#ff9f1a'), ('原创榜', '#7b61ff')]):
        r = rng('chart', i)
        ids = r.sample(range(1, len(SONG_LIST) + 1), 20)
        songs = []
        for rank, sid in enumerate(ids, 1):
            delta = r.choice([0, 0, 1, 2, 3, -1, -2, 5])
            songs.append(dict(SONG_BY_ID[sid], rank=rank, trend=delta))
        out.append({'id': i + 1, 'name': name, 'color': color, 'updateText': '每天更新' if i < 2 else '每周四更新',
                    'cover': cover(i * 3), 'songs': songs})
    return out


def banners():
    texts = [('新歌首发', '林夕语《晚风告白》全网独家', 0), ('歌单推荐', '深夜书房 · 安静陪你读完一本书', 1),
             ('影视原声', '《星际回响》原声大碟上线', 2)]
    out = []
    for i, (title, sub, k) in enumerate(texts):
        target = {'type': 'song', 'id': 1} if k == 0 else {'type': 'playlist', 'id': 1 if k == 1 else 102}
        out.append({'id': f'ub{i + 1}', 'image': img('banner', f'music{i + 1}'), 'title': title, 'subtitle': sub,
                    'colors': [COVER_COLORS[i * 3], COVER_COLORS2[i * 5]], 'target': target})
    return out


def tags():
    return [{'name': t, 'count': len(list_playlists(t))} for t in PLAYLIST_TAGS]
