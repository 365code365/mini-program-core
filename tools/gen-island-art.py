#!/usr/bin/env python3
"""给《萌宠小岛》（sample-game/play-3d-island）用豆包 Seedream 生成美术，并加工成游戏能直接用的图集。

    ARK_API_KEY=… python3 tools/gen-island-art.py            # 生成缺的图 + 重新加工
    python3 tools/gen-island-art.py --process-only            # 只用缓存的原图重新加工（不调接口）
    ARK_API_KEY=… python3 tools/gen-island-art.py --redo pet_fox,icon_gem   # 重新生成指定几张

密钥只从环境变量 ARK_API_KEY 读，不写进任何文件。原图缓存在 target/island-art/raw/（不入库），重跑不重复花钱。

产物（sample-game/play-3d-island/art/，进包）：
    textures.jpg   2048×2048 材质图集（4×4 格，每格 512）：草地、沙滩、崖壁、石板、木板、瓦片、墙面、岩石、田地、树叶…（已做无缝处理）
    icons.webp     1024×1024 图标图集（8×8 格，每格 128，带透明）：资源与界面图标（抠掉纯绿底）
    portraits.webp 宠物与建筑立绘（6 列，每格 256，带透明）
    cover.jpg      720×1280 标题 / 加载页
    manifest.json  每张小图在图集里的位置
另外覆盖 play-3d-island/icon.png（144×144 商店图标，从封面的岛截出来）。

为什么不用 Python 自带的 urllib：本机 Python 3.7 的证书库不全，HTTPS 握手失败；curl 走系统证书。
"""
import concurrent.futures as cf
import json
import os
import subprocess
import sys
import time

import numpy as np
from PIL import Image, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW = os.path.join(ROOT, 'target', 'island-art', 'raw')
OUT = os.path.join(ROOT, 'sample-game', 'play-3d-island', 'art')
API = 'https://ark.cn-beijing.volces.com/api/v3/images/generations'
MODEL = 'doubao-seedream-4-5-251128'

TEX = '无缝平铺材质贴图（seamless tileable texture），像扫描仪扫出来的平面照片，镜头垂直正对表面，完全没有透视和消失点，没有地平线，纹理在整个画面均匀分布、大小一致，满画面铺满，没有边框，没有阴影，光照均匀，色彩明快，高细节，风格化写实手游 PBR 纹理，'
CUT = '，单个物体居中，完整显示不裁切，物体周围留出空白，纯色亮绿色背景（#00FF00），背景没有任何阴影和渐变，3D 卡通渲染，皮克斯风格，柔和全局光照，边缘清晰，高品质手游美术'
PET = '，Q 版萌宠手办，大眼睛，圆滚滚的身体，开心的表情，站立全身像，正面四分之三视角' + CUT
BLD = '，等距视角 45 度俯视的低多边形风格建筑，立在一小块方形草地底座上，精致手游美术' + CUT

ASSETS = [
    # ───────── 材质（textures.jpg 的 16 格，顺序即格号） ─────────
    ('tex_grass', TEX + '翠绿的短草坪，夹杂少量小花和三叶草'),
    ('tex_sand', TEX + '细腻的浅金色沙滩沙子，少量贝壳碎片'),
    ('tex_cliff', TEX + '泥土与岩石的崖壁侧面，棕色土层，夹杂小石块和草根'),
    ('tex_cobble', TEX + '浅灰色圆润鹅卵石铺地，石缝里有一点青苔'),
    ('tex_planks', TEX + '一排排平行的竖直浅色原木木板，每块木板宽度相同、互相平行，木纹清晰，木板两端有钉子'),
    ('tex_roof', TEX + '浅灰白色陶瓦屋顶瓦片，整齐排列的鱼鳞瓦'),
    ('tex_plaster', TEX + '正对着拍的米白色抹灰墙面平面，细小均匀的颗粒纹理，轻微斑驳'),
    ('tex_rock', TEX + '灰色花岗岩岩石表面，粗糙，有裂纹'),
    ('tex_field', TEX + '整齐的农田垄沟，深棕色湿润的土壤，一行行嫩绿的麦苗'),
    ('tex_leaves', TEX + '茂密的绿色树叶冠层，从上往下看，阳光照在叶子上'),
    ('tex_bark', TEX + '树皮纹理，棕色，竖向纹路'),
    ('tex_hay', TEX + '金黄色干草堆的表面，稻草纹理'),
    ('tex_brick', TEX + '正对着拍的暖红色砖墙平面，砖块横平竖直整齐错缝排列，灰色砖缝'),
    ('tex_hillgrass', TEX + '从正上方往下看的深绿色茂密草地，草叶均匀，点缀一点白色小野花'),
    ('tex_metal', TEX + '略带划痕的浅灰色金属板，铆钉'),
    # ───────── 图标（icons.webp） ─────────
    ('icon_coin', '一枚闪亮的金币，正面刻着五角星，厚实的立体边缘，游戏图标' + CUT),
    ('icon_gem', '一颗闪闪发光的蓝色钻石宝石，多切面，游戏图标' + CUT),
    ('icon_wheat', '一束金黄饱满的麦穗，用小绳捆着，游戏图标' + CUT),
    ('icon_wood', '三根堆在一起的原木，切面露出年轮，游戏图标' + CUT),
    ('icon_stone', '一堆灰色的石块，游戏图标' + CUT),
    ('icon_bread', '一条刚出炉的金黄色法式面包，表面有切口，游戏图标' + CUT),
    ('icon_fish', '一条新鲜的蓝色大鱼，游戏图标' + CUT),
    ('icon_honey', '一罐金黄色的蜂蜜，罐口盖着布扎着绳子，旁边有蜂蜜勺，游戏图标' + CUT),
    ('icon_xp', '一颗金色发光的五角星徽章，游戏图标' + CUT),
    ('icon_hammer', '一把木柄铁锤，游戏建造图标' + CUT),
    ('icon_egg', '一颗带彩色波点和花纹的魔法宠物蛋，微微发光，游戏图标' + CUT),
    ('icon_gift', '一个粉红色系金色缎带蝴蝶结的礼物盒，游戏图标' + CUT),
    ('icon_bag', '一个紫色的购物袋，上面有金色星星，游戏商店图标' + CUT),
    ('icon_scroll', '一卷展开的羊皮纸任务卷轴，红色蜡封，游戏图标' + CUT),
    ('icon_paw', '一个粉色的可爱猫爪印，游戏图标' + CUT),
    ('icon_calendar', '一本可爱的台历，上面写着数字 7，蓝色顶部，游戏签到图标' + CUT),
    ('icon_gear', '一个银灰色的齿轮，游戏设置图标' + CUT),
    ('icon_video', '一个红色圆角矩形里有白色播放三角，游戏看视频图标' + CUT),
    ('icon_share', '一个绿色圆形分享图标，三个白色圆点连线' + CUT),
    ('icon_boost', '一个橙黄色圆形里有一道白色闪电，游戏加速图标' + CUT),
    ('icon_crown', '一顶镶着红蓝宝石的金色皇冠，游戏图标' + CUT),
    ('icon_lock', '一把金色的挂锁，游戏图标' + CUT),
    ('icon_clock', '一个白色表盘蓝色边框的圆形闹钟，游戏图标' + CUT),
    ('icon_island', '一座漂浮在蓝色海面上的小岛，岛上有棕榈树和红顶小屋，游戏图标' + CUT),
    # ───────── 宠物立绘（与 defs.js 的 PETS 一一对应） ─────────
    ('pet_chick', '一只嫩黄色的小鸡，橙色小嘴' + PET),
    ('pet_bunny', '一只白色的小兔子，粉色的长耳朵' + PET),
    ('pet_pup', '一只橘黄色的柴犬幼犬，咧嘴笑' + PET),
    ('pet_piggy', '一只粉色的小猪，卷卷的尾巴' + PET),
    ('pet_cat', '一只胖胖的橘猫，抱着一条小鱼' + PET),
    ('pet_panda', '一只熊猫宝宝，抱着一根竹子' + PET),
    ('pet_frog', '一只绿色的小青蛙，头顶一片荷叶' + PET),
    ('pet_fox', '一只火红色的小狐狸，蓬松的大尾巴，尾巴尖是白色' + PET),
    ('pet_owl', '一只戴着小圆眼镜和学士帽的棕色猫头鹰' + PET),
    ('pet_unicorn', '一只彩虹鬃毛的白色独角兽小马，金色的角，周围有星星闪光，传说级稀有' + PET),
    ('pet_dragon', '一只金色鳞片的小龙宝宝，粉色小翅膀，周围有金光，传说级稀有' + PET),
    # ───────── 建筑立绘（与 defs.js 的 BUILD_ORDER 一一对应） ─────────
    ('bld_farm', '一片金黄的麦田，旁边有红顶小农舍和白色风车' + BLD),
    ('bld_lumber', '伐木场，绿色屋顶的木屋，旁边堆着原木和圆锯' + BLD),
    ('bld_quarry', '采石场，灰色的大石头，木架子和装满石头的小矿车' + BLD),
    ('bld_warehouse', '紫色的木质仓库，大门上有白色交叉木条，门口有箱子' + BLD),
    ('bld_market', '热闹的集市摊位，三个红白蓝条纹遮阳棚，摆满水果蔬菜' + BLD),
    ('bld_bakery', '粉色墙面红色屋顶的面包坊，烟囱冒着白烟，橱窗里有面包' + BLD),
    ('bld_dock', '海边的小渔港，木栈道码头，一艘红色小渔船，蓝顶小屋' + BLD),
    ('bld_pethouse', '圆形的粉色宠物小屋，粉色圆锥屋顶，白色小栅栏，门口有骨头和毛线球' + BLD),
    ('bld_lighthouse', '红白条纹的灯塔，顶部发着暖黄色的光' + BLD),
    ('bld_apiary', '一个小小的养蜂场：三个黄色木头蜂箱紧挨着摆在一起，蜂箱旁边一小丛五颜六色的花，几只小蜜蜂，所有东西都在同一块方形草地底座上' + BLD),
    # ───────── 封面 ─────────
    ('cover', '手机游戏《萌宠小岛》的标题加载页，竖屏，一座漂浮在碧蓝大海上的可爱小岛，岛上有金色麦田、白色风车、红顶小屋、红白灯塔、'
              '椰子树，小鸡、柴犬、小兔子、熊猫等 Q 版萌宠在岛上玩耍，天空湛蓝有大朵白云，阳光明媚，画面上方留出空白给标题，'
              '画面下方三分之一是平静的海面留给加载进度条，3D 卡通渲染，皮克斯风格，色彩明快饱和，高品质手游宣传图，不要任何文字', '1440x2560'),
]
ASSET_IDS = [a[0] for a in ASSETS]


def generate(aid, prompt, size):
    raw = os.path.join(RAW, aid + '.jpg')
    body = json.dumps({'model': MODEL, 'prompt': prompt, 'sequential_image_generation': 'disabled', 'response_format': 'url',
                       'size': size, 'stream': False, 'watermark': False}, ensure_ascii=False)
    for attempt in range(3):
        t = time.time()
        # 密钥经 curl 的 -H @- 从 stdin 读：不出现在进程参数里（ps 看不到）
        r = subprocess.run(['curl', '-sS', '-m', '180', '-X', 'POST', API, '-H', 'Content-Type: application/json', '-H', '@-', '--data-binary', body],
                           input=('Authorization: Bearer ' + os.environ['ARK_API_KEY'] + '\n').encode(), capture_output=True)
        try:
            res = json.loads(r.stdout.decode('utf-8', 'replace'))
            url = res['data'][0]['url']
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


# ───────── 加工 ─────────
def seamless(img, size=512):
    """无缝化：分两遍——先横向：把左右错开半格的自己按「离左右边多近」混进来（错开那份的接缝在正中竖线上，那里权重是 0）；
    再对结果纵向做一遍。一遍同时混两个方向的话，错开那份的十字接缝会露在四条边的中点上"""
    a = np.asarray(img.convert('RGB').resize((size, size), Image.LANCZOS)).astype(np.float32)
    t = np.arange(size, dtype=np.float32)
    w = np.clip(np.minimum(t, size - 1 - t) / (size * 0.22), 0, 1)        # 边上 0、离边 22% 以上 1
    w = w * w * (3 - 2 * w)                                               # smoothstep
    b = np.roll(a, size // 2, 1)
    a = a * w[None, :, None] + b * (1 - w[None, :, None])
    b = np.roll(a, size // 2, 0)
    a = a * w[:, None, None] + b * (1 - w[:, None, None])
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8))


# 材质的「风格化」：AI 出的是写实照片级的纹理，直接贴在卡通场景里又暗又花。按格子把平均色挪到原来手调的卡通色上、
# 把对比度压下来（细节还在，只是不抢戏）。目标色取自 js/world/terrain.js / models.js 原来的配色
STYLE = {
    'grass': ((143, 209, 106), 0.55), 'hillgrass': ((111, 191, 90), 0.55), 'sand': ((242, 223, 167), 0.6), 'cliff': ((150, 112, 76), 0.7),
    'field': ((140, 96, 58), 0.75), 'leaves': ((108, 197, 81), 0.6), 'hay': ((233, 196, 106), 0.65),
}
def stylize(tile, name):
    if name not in STYLE:
        # 建筑用的中性材质（瓦、墙、石、木、金属）会被顶点色染色：平均亮度提到 ~215，染出来才是原来的颜色、不发灰；
        # v4：整体色偏全部去掉（只留明暗细节），对比度压到 0.6。v3 留了一半色偏（木板偏棕、瓦偏灰），
        # 再乘上顶点色（红谷仓、紫仓库）就成了发黑的泥色 —— 颜色只该由顶点色决定，贴图只给纹理细节
        a = np.asarray(tile).astype(np.float32); lum_px = a.mean(2, keepdims=True); lum = lum_px.mean()
        detail = (lum_px - lum) * 0.6
        return Image.fromarray(np.clip(np.repeat(225.0 + detail * (225.0 / max(lum, 1)), 3, 2), 0, 255).astype(np.uint8))
    target, contrast = STYLE[name]
    a = np.asarray(tile).astype(np.float32); m = a.mean((0, 1))
    a = np.array(target, np.float32) + (a - m) * contrast
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8))


def cutout(img, size):
    """抠底：模型画出来的「绿底」并不是纯 #00FF00，是带渐变、带投影的黄绿色，所以
    1. 背景色 = 这张图四条边的中位色（每张图自己估）；
    2. 距离用色度（r、g 各除以亮度）为主、亮度为辅：同色相的投影（暗一点的绿）也算背景；
    3. 只去掉**从边上连通过来**的背景（洪水填充）：物体本身是绿的（青蛙、叶子）也不会被挖掉；
    4. 边缘按距离羽化 2 像素，并把边上的背景色溢色扣掉。
    最后裁到物体包围盒，等比放进 size×size（留 6% 边）。"""
    from scipy import ndimage
    im0 = img.convert('RGB')
    if max(im0.size) > 1024:                                   # 先缩到 1024：抠图更快，结果也够 256 用
        k = 1024 / max(im0.size); im0 = im0.resize((int(im0.width * k), int(im0.height * k)), Image.LANCZOS)
    a = np.asarray(im0).astype(np.float32)
    h, w, _ = a.shape
    border = np.concatenate([a[:6].reshape(-1, 3), a[-6:].reshape(-1, 3), a[:, :6].reshape(-1, 3), a[:, -6:].reshape(-1, 3)])
    bg = np.median(border, 0)
    # 魔棒：背景是「平滑」的（渐变、光晕、投影都平滑），物体轮廓是「陡」的。
    # 梯度小的像素连成片、且连到画布边上的那几片 = 背景；按颜色阈值抠在这种渐变底上要么抠不干净要么挖穿物体
    sm = ndimage.gaussian_filter(a, (1.6, 1.6, 0))
    grad = sum(np.hypot(ndimage.sobel(sm[..., c], 0), ndimage.sobel(sm[..., c], 1)) for c in range(3))
    flat = grad < 34
    # 离背景色太远的（白色高光、深色物体）即使平也不算背景：物体内部大片平涂不会因为挨着边被吃掉
    far = np.sqrt(((a - bg) ** 2).sum(2)) > 150
    lab, n = ndimage.label(flat & ~far)
    edge = np.unique(np.concatenate([lab[0], lab[-1], lab[:, 0], lab[:, -1]]))
    bgmask = np.isin(lab, edge[edge > 0])
    bgmask = ndimage.binary_closing(bgmask, iterations=2)                 # 背景里的小噪点洞补上
    fg = ~bgmask
    fg = ndimage.binary_opening(fg, iterations=2)                         # 去掉背景上残留的小碎块
    lab2, n2 = ndimage.label(fg)
    if n2 > 1:                                                             # 只留面积够大的块（主体 + 挨着的部件）
        sizes = ndimage.sum(fg, lab2, range(1, n2 + 1))
        keep = np.isin(lab2, 1 + np.where(sizes >= sizes.max() * 0.04)[0])
        fg = keep
    # 物体内部的洞：颜色像背景的（齿轮中间、锁扣里的空）留透明，不像的（误判）补上
    holes = ndimage.binary_fill_holes(fg) & ~fg
    hl, hn = ndimage.label(holes)
    for i in range(1, hn + 1):
        m = hl == i
        mc = a[m].mean(0)
        greenish = mc[1] > mc[0] + 25 and mc[1] > mc[2] + 25          # 洞里是绿底（齿轮、锁扣中间）：留透明
        if not greenish and np.sqrt(((mc - bg) ** 2).sum()) > 70: fg |= m
    # 边缘：往里吃 1 像素（去掉混着背景色的那圈），再羽化
    fg = ndimage.binary_erosion(fg, iterations=1)
    alpha = ndimage.gaussian_filter(fg.astype(np.float32), 1.0)
    alpha = np.clip((alpha - 0.15) / 0.7, 0, 1)
    # 去溢色：半透明边上的像素把背景色按透明度减掉一部分
    k = (1 - alpha)[..., None]
    rgb = np.clip((a - bg * k * 0.85) / np.maximum(1 - k * 0.85, 0.15), 0, 255)
    rgba = np.dstack([rgb, alpha * 255]).astype(np.uint8)
    im = Image.fromarray(rgba, 'RGBA')
    box = im.getbbox() or (0, 0, im.width, im.height)
    im = im.crop(box)
    s = size * 0.88 / max(im.width, im.height)
    im = im.resize((max(1, int(im.width * s)), max(1, int(im.height * s))), Image.LANCZOS)
    out = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    out.paste(im, ((size - im.width) // 2, (size - im.height) // 2), im)
    return out


def process():
    os.makedirs(OUT, exist_ok=True)
    have = lambda aid: os.path.isfile(os.path.join(RAW, aid + '.jpg'))
    load = lambda aid: Image.open(os.path.join(RAW, aid + '.jpg'))
    manifest = {'textures': {}, 'icons': {}, 'portraits': {}}
    # 材质：4×4，最后一格是纯白（不贴图的零件采样它，只吃顶点色）
    tex = [a[0] for a in ASSETS if a[0].startswith('tex_')]
    atlas = Image.new('RGB', (2048, 2048), (255, 255, 255))
    for i, aid in enumerate(tex):
        if not have(aid):
            continue
        atlas.paste(stylize(seamless(load(aid)), aid[4:]), ((i % 4) * 512, (i // 4) * 512))
        manifest['textures'][aid[4:]] = i
    manifest['textures']['white'] = 15
    atlas.save(os.path.join(OUT, 'textures.jpg'), quality=86, optimize=True, progressive=False)
    # 图标：8×8 × 128
    icons = [a[0] for a in ASSETS if a[0].startswith('icon_')]
    ia = Image.new('RGBA', (1024, 1024), (0, 0, 0, 0))
    for i, aid in enumerate(icons):
        if not have(aid):
            continue
        ia.paste(cutout(load(aid), 128), ((i % 8) * 128, (i // 8) * 128))
        manifest['icons'][aid[5:]] = [(i % 8) * 128, (i // 8) * 128]
    ia.save(os.path.join(OUT, 'icons.webp'), quality=88, method=6)
    # 立绘：6 列 × 256
    ports = [a[0] for a in ASSETS if a[0].startswith(('pet_', 'bld_'))]
    rows = (len(ports) + 5) // 6
    pa = Image.new('RGBA', (6 * 256, rows * 256), (0, 0, 0, 0))
    for i, aid in enumerate(ports):
        if not have(aid):
            continue
        pa.paste(cutout(load(aid), 256), ((i % 6) * 256, (i // 6) * 256))
        manifest['portraits'][aid] = [(i % 6) * 256, (i // 6) * 256]
    pa.save(os.path.join(OUT, 'portraits.webp'), quality=86, method=6)
    manifest['portraitSize'] = 256
    manifest['iconSize'] = 128
    if have('cover'):
        cv = load('cover').convert('RGB').resize((720, 1280), Image.LANCZOS)
        cv.save(os.path.join(OUT, 'cover.jpg'), quality=84, optimize=True)
        manifest['cover'] = True
        # 商店图标：封面中间偏下的岛那一块，方形裁切 + 圆角
        isl = cv.crop((90, 360, 630, 900)).resize((144, 144), Image.LANCZOS).convert('RGBA')
        mask = Image.new('L', (144 * 4, 144 * 4), 0)
        from PIL import ImageDraw
        ImageDraw.Draw(mask).rounded_rectangle((0, 0, 144 * 4 - 1, 144 * 4 - 1), radius=32 * 4, fill=255)
        isl.putalpha(mask.resize((144, 144), Image.LANCZOS))
        isl.save(os.path.join(ROOT, 'sample-game', 'play-3d-island', 'icon.png'), optimize=True)
    manifest['generatedBy'] = MODEL
    json.dump(manifest, open(os.path.join(OUT, 'manifest.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    sizes = {f: os.path.getsize(os.path.join(OUT, f)) for f in os.listdir(OUT)}
    print('产物：' + '，'.join(f'{k} {v // 1024}KB' for k, v in sorted(sizes.items())))
    missing = [a for a in ASSET_IDS if not have(a)]
    if missing:
        print('⚠ 还缺：' + ', '.join(missing))


def main():
    os.makedirs(RAW, exist_ok=True)
    args = sys.argv[1:]
    if '--process-only' not in args:
        if not os.environ.get('ARK_API_KEY'):
            sys.exit('要设 ARK_API_KEY（火山方舟的 API Key）')
        redo = set()
        if '--redo' in args:
            redo = set(args[args.index('--redo') + 1].split(','))
        todo = [a for a in ASSETS if a[0] in redo or not os.path.isfile(os.path.join(RAW, a[0] + '.jpg'))]
        print(f'要生成 {len(todo)} 张（已缓存 {len(ASSETS) - len(todo)} 张）', flush=True)
        with cf.ThreadPoolExecutor(max_workers=5) as ex:
            list(ex.map(lambda a: generate(a[0], a[1], a[2] if len(a) > 2 else '2K'), todo))
    process()


if __name__ == '__main__':
    main()
