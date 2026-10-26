"""演示数据接口的公共部分：确定性随机、分页、图片地址、人名 / 头像池。

为什么数据用「规则生成」而不是存一份 JSON：几百部电影的短评、几十个歌单的歌曲、
上百张照片，手写不现实，存文件又要维护两份（数据 + 生成图片的脚本各读一遍）。
规则生成 + 固定种子 = 同一请求永远同样结果（示例小程序的截图基线可复现），
而 make_demo_assets.py 直接 import 这几个模块拿标题与配色去画图，两边天然一致。
"""
import random

# 资源池的大小：make_demo_assets.py 按这几个数生成文件，数据里引用的编号不会越界
N_AVATARS = 24
N_STILLS = 24
N_COVERS = 30
N_AUDIO = 3
AUDIO_SECONDS = 20          # 每段试听音频的长度（歌词时间轴按它排）；16kHz 16bit 单声道 20 秒 ≈ 640KB

# 头像：一个字 + 底色（画在 96×96 的圆形头像上）
AVATAR_SPECS = [
    ('林', '#5B8FF9'), ('苏', '#F6903D'), ('陈', '#61DDAA'), ('周', '#7262FD'), ('许', '#F08BB4'),
    ('沈', '#78D3F8'), ('顾', '#9661BC'), ('江', '#F6BD16'), ('叶', '#2EC7C9'), ('程', '#E8684A'),
    ('白', '#6DC8EC'), ('宋', '#FF9D4D'), ('唐', '#269A99'), ('韩', '#FF99C3'), ('温', '#5D7092'),
    ('陆', '#BFBFBF'), ('秦', '#D4380D'), ('乔', '#13A8A8'), ('夏', '#FAAD14'), ('梁', '#2F54EB'),
    ('何', '#EB2F96'), ('孟', '#52C41A'), ('谢', '#722ED1'), ('姜', '#FA541C'),
]

SURNAMES = '林苏陈周许沈顾江叶程白宋唐韩温陆秦乔夏梁何孟谢姜'
GIVEN = ['子轩', '雨桐', '一鸣', '若曦', '浩然', '思远', '清欢', '知夏', '景行', '南乔', '星河', '牧之',
         '书瑶', '承泽', '晚晴', '亦凡', '时雨', '安然', '听澜', '可欣', '明哲', '沐阳', '语嫣', '慕白']
NICK_A = ['爱吃', '不睡觉的', '路过的', '认真的', '迷路的', '佛系', '早起的', '快乐的', '晚风里的', '会飞的']
NICK_B = ['小熊', '橘猫', '影迷', '咸鱼', '星星', '柚子', '狐狸', '鲸鱼', '旅人', '番茄', '少年', '兔子']


def rng(*seed):
    """按种子造一个独立的随机源：同样的种子永远同样的序列（与调用顺序、进程无关）"""
    return random.Random('|'.join(str(s) for s in seed))


def person(r):
    return r.choice(SURNAMES) + r.choice(GIVEN)


def nickname(r):
    return r.choice(NICK_A) + r.choice(NICK_B) + (str(r.randint(1, 999)) if r.random() < 0.4 else '')


def img(kind, name):
    """站内相对地址；demo_api 发出去之前统一补成 http://<Host> 的绝对地址"""
    return f'/api/demo/img/{kind}/{name}.jpg'


def avatar(i):
    return img('avatar', f'a{(i % N_AVATARS) + 1:02d}')


def still(i):
    return img('still', f's{(i % N_STILLS) + 1:02d}')


def cover(i):
    return img('cover', f'c{(i % N_COVERS) + 1:02d}')


def audio(i):
    return f'/api/demo/audio/{(i % N_AUDIO) + 1}.wav'


def to_int(v, default, lo, hi):
    try:
        n = int(v)
    except (TypeError, ValueError):
        return default
    return max(lo, min(hi, n))


def paginate(items, query, default_size=10):
    """所有列表接口同一个形状：{list, page, size, total, hasMore}。size 上限 50（别一次拉爆小程序的 setData）"""
    page = to_int(query.get('page'), 1, 1, 10_000)
    size = to_int(query.get('size'), default_size, 1, 50)
    start = (page - 1) * size
    chunk = items[start:start + size]
    return {'list': chunk, 'page': page, 'size': size, 'total': len(items), 'hasMore': start + len(chunk) < len(items)}


def absolutize(obj, base):
    """把返回里所有 `/api/demo/` 开头的字符串补成绝对地址。

    为什么在出口统一做：数据模块在 import 时就生成好、被多个请求共享，不能带某次请求的 Host；
    而小程序里 <image src> / InnerAudioContext.src 要绝对地址（包内相对路径会被当成包里的文件）"""
    if isinstance(obj, str):
        return base + obj if obj.startswith('/api/') else obj
    if isinstance(obj, list):
        return [absolutize(x, base) for x in obj]
    if isinstance(obj, dict):
        return {k: absolutize(v, base) for k, v in obj.items()}
    return obj


def fmt_count(n):
    return f'{n / 10000:.1f}万' if n >= 10000 else str(n)
