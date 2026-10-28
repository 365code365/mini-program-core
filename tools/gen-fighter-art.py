#!/usr/bin/env python3
"""给《拳魂》（sample-game/play-fighter，横屏格斗）用豆包 Seedream 4.5 生成美术，并加工成游戏能直接用的图集。
    ARK_API_KEY=… python3 tools/gen-fighter-art.py              # 生成缺的图 + 重新加工
    python3 tools/gen-fighter-art.py --process-only              # 只用缓存的原图重新加工（不调接口）
    ARK_API_KEY=… python3 tools/gen-fighter-art.py --redo yan_hk,shuang_up   # 重画指定几张
密钥只从环境变量 ARK_API_KEY 读，不写进任何文件（curl 从 stdin 读请求头，ps 里看不到）。
原图缓存在 target/fighter-art/raw/（不入库），重跑不重复花钱。

怎么让同一个角色的十几个动作长得一样：先文生图出一张「设定图」（<角色>_ref），之后每个动作都把设定图作为参考图
（image 参数，base64）喂进去做图生图，提示词只描述动作。实测脸、发型、服装、配色都能保持一致。
动作图的人物大小每张不一样（模型按画面构图），所以加工时不按原图比例，而是按每个动作的「身高系数」把包围盒缩到统一的身高。

产物（sample-game/play-fighter/art/，进包）：
    <角色>.webp     该角色全部动作的图集（带透明，抠掉亮绿底），2048 以内
    stage_*.jpg     两张场景（1600×900）
    cover.jpg       标题页（1280×720）
    manifest.json   每个动作在图集里的矩形、脚底锚点、逻辑身高
另外覆盖 play-fighter/icon.png（144×144，从封面截）。
"""
import base64
import concurrent.futures as cf
import io
import json
import os
import subprocess
import sys
import time

import numpy as np
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW = os.path.join(ROOT, 'target', 'fighter-art', 'raw')
OUT = os.path.join(ROOT, 'sample-game', 'play-fighter', 'art')
API = 'https://ark.cn-beijing.volces.com/api/v3/images/generations'
MODEL = 'doubao-seedream-4-5-251128'

STYLE = '2D 格斗游戏角色精灵原画，90 年代日式街机格斗游戏风格的高清重制版，精致手绘动画赛璐璐上色，强烈的明暗对比与边缘高光。'
BG = ('纯色亮绿色背景（#00FF00），背景没有任何阴影、地面和渐变，单个角色，镜头拉远，人物只占画面高度的六成、四周留出大片空白，'
      '全身完整显示不裁切，头顶和脚底完整可见，没有动作残影、速度线和任何特效，画面里没有文字和其他物体')
SAME = '参考图里的同一个角色（同样的脸、发型、头饰、服装、配色、画风完全一致），2D 格斗游戏精灵，身体侧向画面右侧。动作：'

CHARS = {
    'yan': STYLE + '角色：二十岁的热血青年武术家，黑色短发略微凌乱，额头绑着长长的红色头带（带尾飘动），黑色无袖短夹克敞开、里面白色紧身背心，'
                   '深红色宽松武道裤，黑色护腕和绑腿，赤手，结实的肌肉。姿势：格斗准备架势，身体侧向画面右侧，双拳举起护在胸前，双脚前后分开站稳。' + BG,
    'shuang': STYLE + '角色：二十岁的冷艳女武术家，银白色长发高高束成马尾，冰蓝色眼睛，额前一缕刘海，深蓝色与银色相间的改良旗袍式短上衣（立领、盘扣），'
                      '白色短披肩飘带，黑色紧身裤与蓝色护膝，白色绑带格斗靴，银色护手，身材修长矫健，衣着端庄得体。'
                      '姿势：格斗准备架势，身体侧向画面右侧，左掌在前右拳收在腰间，双脚前后分开站稳。' + BG,
}

# (动作, 提示词, 身高系数 = 包围盒高度 / 站立身高, 锚点：feet = 脚底那几行的中心，center = 包围盒中心)
POSES = [
    ('idle', None, 1.0, 'torso'),
    # 走路循环 4 帧（保持格斗架势往前挪步）：迈步着地 → 过渡 → 换脚着地 → 过渡。锚点取躯干（torso）：
    # 按脚底算锚点时，两腿一前一后分开的那帧锚点会往前跳几十像素，走起来人一抽一抽的
    ('walk1', '格斗架势前进步伐第 1 帧：保持格斗架势、双拳护在胸前，靠近画面右侧的前脚刚向前迈出、脚跟着地，后脚脚尖蹬地，两腿前后大步分开，膝盖微屈，上身直立', 1.0, 'torso'),
    ('walk2', '格斗架势前进步伐第 2 帧：保持格斗架势、双拳护在胸前，一条腿站直承重，另一条腿的脚刚刚离开地面一点点、从支撑腿旁边向前经过，膝盖只微微弯曲、抬得很低（脚离地不超过小腿一半高），两膝靠拢，是普通走路的过渡姿势', 1.02, 'torso'),
    ('walk3', '格斗架势前进步伐第 3 帧：保持格斗架势、双拳护在胸前，两腿前后分开、步幅中等，前腿膝盖弯曲承重，后腿伸直脚跟抬起，上身微微前倾', 0.95, 'torso'),
    ('walk4', '格斗架势前进步伐第 4 帧：保持格斗架势、双拳护在胸前，双脚靠拢几乎并在一起，一只脚着地承重，另一只脚脚尖点地准备迈出，膝盖微屈', 1.0, 'torso'),
    ('walk', '向前迈步行走，前脚刚落地，后脚抬起脚跟，双拳护在胸前，身体微微前倾', 1.0, 'feet'),
    ('crouch', '下蹲防御架势，双膝弯曲深蹲，重心很低，双拳护在面前', 0.62, 'feet'),
    ('jump', '腾空跳起，双膝收到胸前蜷起身体，双拳护在面前，双脚离地', 0.72, 'center'),
    ('lp', '快速刺拳：前手直拳笔直伸向画面右侧、手臂完全伸直，后手护住下巴，双脚前后站稳', 1.0, 'feet'),
    ('hp', '全力重拳：后手冲拳向画面右侧打出、手臂完全伸直，身体大幅前倾转腰，前腿弓步，表情凶狠', 0.95, 'feet'),
    ('lk', '中段前踢：前腿向画面右侧水平踢出、腿伸直与腰同高，支撑腿站稳，双拳护在胸前', 1.0, 'feet'),
    ('hk', '高位踢腿：一条腿高高踢向画面右侧、脚尖与头同高，支撑腿站稳，双臂张开保持平衡', 1.12, 'feet'),
    ('ck', '下蹲扫腿，远景全身：身体压低蹲在地上，一只手撑地，一条腿贴着地面向画面右侧伸直横扫，整条腿和脚完整可见', 0.5, 'feet'),
    ('jk', '空中飞踢：身体在空中，一条腿向画面右前下方斜着伸直踢出，另一条腿收起，双手握拳', 0.85, 'center'),
    ('sp', '发功推掌：双腿弓步扎稳，双掌并拢手腕相贴，掌心向画面右侧用力推出、手臂伸直，表情专注怒吼', 0.92, 'feet'),
    ('up', '升龙冲天拳：身体腾空向上跃起，一只拳头高高举过头顶向上打出、手臂伸直，另一只手收在腰间，双腿一曲一伸', 1.28, 'center'),
    ('block', '侧身站立格挡，身体侧向画面右侧：双前臂交叉成 X 形竖在脸前挡住来自右侧的攻击，身体微微后仰缩紧，咬牙忍耐，双脚前后分开站稳', 0.97, 'feet'),
    ('hit', '被重击后仰：头和上身向画面左后方猛地仰倒，双臂甩开，痛苦表情，双脚还在地上踉跄', 0.95, 'feet'),
    ('down', '被击倒仰面平躺在地上，从侧面看，头朝画面左侧，脚朝画面右侧，双腿伸直平放贴着地面，双臂摊在身体两侧，整个身体是一条水平线', 0.36, 'feet'),
    ('win', '胜利姿势：自信地站立，一只拳头高举向天，另一只手叉腰，表情得意微笑', 1.1, 'feet'),
]
STAND_H = 500            # 图集里站立身高（像素）。横屏手机上人物约 190 逻辑像素高 × dpr 3 ≈ 570，双线性放大 1.3 倍看不出糊

STAGES = {
    'stage_city': '2D 格斗游戏的横版战斗场景背景，90 年代日式街机格斗游戏风格的高清重制版，精致手绘，夜晚下雨后的中国城老街，'
                  '两侧是挂满红色灯笼和霓虹灯招牌的旧楼，远处高楼林立，招牌上的字是模糊的光斑，街道远处有模糊的围观人群剪影，'
                  '画面下方四分之一是宽阔平整、湿漉漉反光的石板路面，路面延伸到画面左右两端，空旷没有任何人和物体，'
                  '镜头平视，冷暖对比强烈，紫色与青色霓虹，景深，电影级光影，不要文字，不要任何角色',
    'stage_temple': '2D 格斗游戏的横版战斗场景背景，90 年代日式街机格斗游戏风格的高清重制版，精致手绘，黄昏的山顶古寺庭院，'
                    '远处是层层叠叠的山峦与云海，金色夕阳，飞檐斗拱的红色木构大殿，两侧盛开的樱花树，花瓣飘落，石灯笼，'
                    '画面下方四分之一是宽阔平整的青石板地面，地面延伸到画面左右两端，空旷没有任何人和物体，'
                    '镜头平视，暖色调，体积光，丁达尔光束，景深，电影级光影，不要文字，不要任何角色',
}
COVER = ('横版格斗游戏的标题宣传画，参考图 1 和参考图 2 里的两位角色（脸、发型、服装、配色完全一致）面对面对峙，'
         '左边是参考图 1 的红头带青年，拳头燃起橙红色火焰；右边是参考图 2 的银发女武术家，手掌缠绕冰蓝色闪电；'
         '两人中间火焰与冰电碰撞迸发出耀眼的光芒和火花，背景是暗红到深蓝的对比色天空与飞散的碎石，'
         '动感，强烈的视觉冲击力，电影级光影，赛璐璐上色，90 年代日式街机格斗游戏宣传画风格，画面上方中间留出空白给标题，不要任何文字')


def ref_data(path):
    im = Image.open(path).convert('RGB')
    im.thumbnail((1024, 1024), Image.LANCZOS)
    b = io.BytesIO()
    im.save(b, 'JPEG', quality=90)
    return 'data:image/jpeg;base64,' + base64.b64encode(b.getvalue()).decode()


def generate(aid, prompt, size, refs=None):
    raw = os.path.join(RAW, aid + '.jpg')
    req = {'model': MODEL, 'prompt': prompt, 'sequential_image_generation': 'disabled', 'response_format': 'url',
           'size': size, 'stream': False, 'watermark': False}
    if refs:
        req['image'] = refs[0] if len(refs) == 1 else refs
    body = json.dumps(req, ensure_ascii=False)
    for attempt in range(3):
        t = time.time()
        r = subprocess.run(['curl', '-sS', '-m', '240', '-X', 'POST', API, '-H', 'Content-Type: application/json', '-H', '@' + hdr_file(),
                            '--data-binary', '@-'], input=body.encode('utf-8'), capture_output=True)
        try:
            url = json.loads(r.stdout.decode('utf-8', 'replace'))['data'][0]['url']
        except Exception:
            print(f'  ✗ {aid} 第 {attempt + 1} 次失败：{r.stdout[:300]!r} {r.stderr[:200]!r}', flush=True)
            time.sleep(3)
            continue
        d = subprocess.run(['curl', '-sS', '-m', '120', '-o', raw + '.part', url], capture_output=True)
        if d.returncode == 0 and os.path.getsize(raw + '.part') > 10000:
            os.replace(raw + '.part', raw)
            print(f'  ✓ {aid}  {time.time() - t:.1f}s', flush=True)
            return True
    return False


_HDR = None


def hdr_file():
    """请求体很大（带 base64 参考图）要走 stdin，所以密钥头改写进一个只有自己能读的临时文件（进程结束删掉）"""
    global _HDR
    if _HDR is None:
        import atexit
        import tempfile
        fd, _HDR = tempfile.mkstemp(prefix='ark-hdr-')
        os.write(fd, ('Authorization: Bearer ' + os.environ['ARK_API_KEY'] + '\n').encode())
        os.close(fd)
        os.chmod(_HDR, 0o600)
        atexit.register(lambda: os.path.exists(_HDR) and os.remove(_HDR))
    return _HDR


# ───────── 加工 ─────────
def key_green(img):
    """抠亮绿底：背景色 = 四条边的中位色；离它近的、且从边上连通过来的像素是背景（角色身上没有绿色，但洞里的绿底也要去掉：
    所以再补一遍「整张图里离背景色很近的」）。边缘羽化 1 像素，最后把所有像素的绿色压到不超过 max(r, b)（去溢色：
    角色配色里没有绿，发丝、飘带边上透出来的绿底就是溢色）"""
    from scipy import ndimage
    a = np.asarray(img.convert('RGB')).astype(np.float32)
    border = np.concatenate([a[:8].reshape(-1, 3), a[-8:].reshape(-1, 3), a[:, :8].reshape(-1, 3), a[:, -8:].reshape(-1, 3)])
    bg = np.median(border, 0)
    dist = np.sqrt(((a - bg) ** 2).sum(2))
    greenish = (a[..., 1] - np.maximum(a[..., 0], a[..., 2])) > 40
    near = (dist < 90) | ((dist < 160) & greenish)
    lab, _ = ndimage.label(near)
    edge = np.unique(np.concatenate([lab[0], lab[-1], lab[:, 0], lab[:, -1]]))
    bgm = np.isin(lab, edge[edge > 0]) | (dist < 45)
    fg = ~bgm
    # 模型常在脚下画一小片橄榄绿的投影，和脚连在一起、洪水填充去不掉；两个角色配色里没有绿，偏绿的一律算背景
    fg &= ~((a[..., 1] - np.maximum(a[..., 0], a[..., 2])) > 14)
    # 发功 / 扫腿时模型爱画黄绿色的气流拖尾：g ≥ r 且远高于 b（皮肤是 r > g，银白是三通道相近）
    fg &= ~((a[..., 1] >= a[..., 0] - 4) & (a[..., 1] - a[..., 2] > 45))
    fg = ndimage.binary_opening(fg, iterations=1)
    lab2, n2 = ndimage.label(fg)
    if n2 > 1:
        sizes = ndimage.sum(fg, lab2, range(1, n2 + 1))
        fg = np.isin(lab2, 1 + np.where(sizes >= sizes.max() * 0.03)[0])
    alpha = ndimage.gaussian_filter(fg.astype(np.float32), 0.9)
    alpha = np.clip((alpha - 0.2) / 0.6, 0, 1)
    rgb = a.copy()
    rgb[..., 1] = np.minimum(rgb[..., 1], np.maximum(rgb[..., 0], rgb[..., 2]) * 1.02 + 4)
    return Image.fromarray(np.dstack([np.clip(rgb, 0, 255), alpha * 255]).astype(np.uint8), 'RGBA')


def pose_sprite(img, ratio, anchor):
    """抠图 → 裁到包围盒 → 缩到身高 STAND_H × ratio → 算脚底锚点（相对精灵左上角的像素）"""
    im = key_green(img)
    box = im.getbbox()
    im = im.crop(box)
    s = STAND_H * ratio / im.height
    im = im.resize((max(1, round(im.width * s)), max(1, round(im.height * s))), Image.LANCZOS)
    al = np.asarray(im)[..., 3]
    if anchor == 'feet':
        rows = al[int(im.height * 0.93):]                  # 最下面 7% 的行：脚（扫腿、躺地也是贴地那一截）
        xs = np.where(rows.max(0) > 128)[0]
        ax = float(xs.mean()) if len(xs) else im.width / 2
        if xs.size and (xs.max() - xs.min()) > im.width * 0.7:
            # 扫腿 / 躺地：贴地的一整截，锚点取躯干（不透明像素的加权中心）
            cols = (al > 128).sum(0)
            ax = float((cols * np.arange(im.width)).sum() / max(cols.sum(), 1))
        ay = float(im.height)
    elif anchor == 'torso':
        # 躯干：身高 25%~50% 那几行（胸口到腰）不透明像素的中心；手臂伸在前面也在这几行里，所以只取每行最长的那段连续像素
        xs = []
        for row in al[int(im.height * 0.25):int(im.height * 0.5)]:
            on = np.where(row > 128)[0]
            if not len(on):
                continue
            runs = np.split(on, np.where(np.diff(on) > 1)[0] + 1)
            r = max(runs, key=len)
            xs.append((r[0] + r[-1]) / 2)
        ax = float(np.median(xs)) if xs else im.width / 2
        ay = float(im.height)
    else:
        cols = (al > 128).sum(0)
        ax = float((cols * np.arange(im.width)).sum() / max(cols.sum(), 1))
        ay = float(im.height) * 0.92
    return im, ax, ay


def pack(sprites, size=2048, pad=4):
    """按高度排序的货架装箱；装不下就整体缩一档再装"""
    order = sorted(sprites.items(), key=lambda kv: -kv[1][0].height)
    scale = 1.0
    while True:
        x = y = shelf = 0
        rects = {}
        ok = True
        for k, (im, _, _) in order:
            w, h = round(im.width * scale), round(im.height * scale)
            if x + w + pad > size:
                x, y, shelf = 0, y + shelf + pad, 0
            if y + h > size:
                ok = False
                break
            rects[k] = (x, y, w, h)
            x += w + pad
            shelf = max(shelf, h)
        if ok:
            return rects, scale
        scale *= 0.94


def process():
    os.makedirs(OUT, exist_ok=True)
    have = lambda aid: os.path.isfile(os.path.join(RAW, aid + '.jpg'))
    load = lambda aid: Image.open(os.path.join(RAW, aid + '.jpg'))
    manifest = {'standH': STAND_H, 'chars': {}, 'generatedBy': MODEL}
    for ch in CHARS:
        sprites = {}
        for pid, _, ratio, anchor in POSES:
            aid = ch + '_' + ('ref' if pid == 'idle' else pid)
            if have(aid):
                sprites[pid] = pose_sprite(load(aid), ratio, anchor)
        if not sprites:
            continue
        rects, sc = pack(sprites)
        atlas = Image.new('RGBA', (2048, 2048), (0, 0, 0, 0))
        poses = {}
        for pid, (x, y, w, h) in rects.items():
            im, ax, ay = sprites[pid]
            if sc != 1.0:
                im = im.resize((w, h), Image.LANCZOS)
            atlas.paste(im, (x, y), im)
            poses[pid] = [x, y, w, h, round(ax * sc, 1), round(ay * sc, 1)]
        # 血条旁的头像：站立图最上面 4% 的行（头顶 / 发髻）的中位列 = 头的水平中心；方框边长 = 身高的 24%
        im = sprites['idle'][0]
        al = np.asarray(im)[..., 3]
        xs = np.where(al[:max(2, int(im.height * 0.04))].max(0) > 128)[0]
        hx = float(np.median(xs)) if len(xs) else im.width / 2
        x, y, w, h = rects['idle']
        hs = h * 0.24
        head = [round(x + hx * sc - hs * 0.5, 1), y, round(hs, 1)]
        bb = atlas.getbbox()
        hgt = min(2048, 1 << int(np.ceil(np.log2(max(bb[3], 1)))))     # 高度裁到 2 的幂，省显存
        atlas = atlas.crop((0, 0, 2048, hgt))
        atlas.save(os.path.join(OUT, ch + '.webp'), quality=90, method=6)
        manifest['chars'][ch] = {'atlas': ch + '.webp', 'w': 2048, 'h': hgt, 'scale': sc, 'poses': poses, 'head': head}
    for sid in STAGES:
        if have(sid):
            load(sid).convert('RGB').resize((1600, 900), Image.LANCZOS).save(os.path.join(OUT, sid + '.jpg'), quality=86, optimize=True)
            manifest.setdefault('stages', []).append(sid)
    if have('cover'):
        cv = load('cover').convert('RGB').resize((1280, 720), Image.LANCZOS)
        cv.save(os.path.join(OUT, 'cover.jpg'), quality=85, optimize=True)
        manifest['cover'] = True
        ic = cv.crop((340, 100, 940, 700)).resize((144, 144), Image.LANCZOS).convert('RGBA')
        mask = Image.new('L', (576, 576), 0)
        ImageDraw.Draw(mask).rounded_rectangle((0, 0, 575, 575), radius=128, fill=255)
        ic.putalpha(mask.resize((144, 144), Image.LANCZOS))
        ic.save(os.path.join(ROOT, 'sample-game', 'play-fighter', 'icon.png'), optimize=True)
    json.dump(manifest, open(os.path.join(OUT, 'manifest.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    sizes = {f: os.path.getsize(os.path.join(OUT, f)) for f in os.listdir(OUT)}
    print('产物：' + '，'.join(f'{k} {v // 1024}KB' for k, v in sorted(sizes.items())))


def jobs(redo):
    """两批：先出设定图（动作图要拿它当参考），再并发出全部动作、场景与封面"""
    need = lambda aid: aid in redo or not os.path.isfile(os.path.join(RAW, aid + '.jpg'))
    first = [(ch + '_ref', p, '2048x2048', None) for ch, p in CHARS.items() if need(ch + '_ref')]
    second = []
    for ch in CHARS:
        for pid, prompt, _, _ in POSES:
            if prompt and need(ch + '_' + pid):
                second.append((ch + '_' + pid, SAME + prompt + '。' + BG, '2048x2048', ch))
    for sid, p in STAGES.items():
        if need(sid):
            second.append((sid, p, '2560x1440', None))
    if need('cover'):
        second.append(('cover', COVER, '2560x1440', 'both'))
    return first, second


def main():
    os.makedirs(RAW, exist_ok=True)
    args = sys.argv[1:]
    if '--process-only' not in args:
        if not os.environ.get('ARK_API_KEY'):
            sys.exit('要设 ARK_API_KEY（火山方舟的 API Key）')
        redo = set(args[args.index('--redo') + 1].split(',')) if '--redo' in args else set()
        first, second = jobs(redo)
        print(f'要生成 {len(first) + len(second)} 张', flush=True)
        run = lambda j: generate(j[0], j[1], j[2], refs(j[3]))
        with cf.ThreadPoolExecutor(max_workers=6) as ex:
            list(ex.map(run, first))
            list(ex.map(run, second))
    process()


_REFS = {}


def refs(which):
    if not which:
        return None
    names = list(CHARS) if which == 'both' else [which]
    out = []
    for n in names:
        if n not in _REFS:
            _REFS[n] = ref_data(os.path.join(RAW, n + '_ref.jpg'))
        out.append(_REFS[n])
    return out


if __name__ == '__main__':
    main()
