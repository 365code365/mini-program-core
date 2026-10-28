#!/usr/bin/env python3
"""给《3D 跑酷》（sample-game/play-3d-runner）用豆包 Seedream 生成美术，加工成游戏直接用的小图。
    ARK_API_KEY=… python3 tools/gen-runner-art.py            # 生成缺的图 + 重新加工
    python3 tools/gen-runner-art.py --process-only            # 只用缓存的原图重新加工（不调接口）
    ARK_API_KEY=… python3 tools/gen-runner-art.py --redo facade_a,train_side   # 重画指定几张
密钥只从环境变量 ARK_API_KEY 读，不写进任何文件。原图缓存在 target/runner-art/raw/（不入库），重跑不重复花钱。
下载、无缝化沿用 tools/gen-island-art.py 的实现（按文件路径加载，不复制一份）。
产物（sample-game/play-3d-runner/art/，进包；缺哪张游戏就退回代码画的那一份）：
    ballast / sleeper / concrete / hazard / metal .jpg   512² 无缝材质（RepeatWrapping 平铺）
    facade_a..d.jpg  512×1024 整栋楼的立面（一张图贴满一面墙，不平铺：窗户网格平铺无缝化会出重影）
    train_side.jpg 1024×256、train_front.jpg 512²   地铁车厢侧面 / 车头
    coin.jpg 256²   金币正面（圆柱端面的 UV 正好是内切圆）
    sky.jpg 1024×512  远景天际线（挂在雾外的一块背板上）；manifest.json 的 fog 是它地平线一带的平均色，雾用这个颜色才接得上
    cover.jpg 720×1280  标题页背景；顺手覆盖 icon.png（144² 商店图标）
"""
import concurrent.futures as cf
import importlib.util
import json
import os
import sys
import numpy as np
from PIL import Image, ImageDraw
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
spec = importlib.util.spec_from_file_location('island_art', os.path.join(ROOT, 'tools', 'gen-island-art.py'))
isl = importlib.util.module_from_spec(spec); spec.loader.exec_module(isl)
RAW = os.path.join(ROOT, 'target', 'runner-art', 'raw')
isl.RAW = RAW                                         # isl.generate 按模块全局 RAW 落盘
OUT = os.path.join(ROOT, 'sample-game', 'play-3d-runner', 'art')
TEX = isl.TEX
FLAT = '正交立面图，镜头正对、完全没有透视，画面被主体完全填满、上下左右都没有留白和背景，光照均匀柔和，风格化写实手游美术，色彩明快饱和，高细节，没有文字和标志，'
FACADE = FLAT + '一整栋城市楼房的正立面，从地面一直到楼顶，底层是带遮阳棚的临街小店，'
ASSETS = [
    ('ballast', TEX + '铁路道床上的碎石，灰色与浅褐色混合的棱角分明的小石子，大小均匀'),
    ('sleeper', TEX + '风化的深棕色旧木板表面，木纹沿水平方向，有细小裂纹'),
    ('concrete', TEX + '浅灰色的站台水泥地砖，方形地砖整齐排列，细缝，略有磨损'),
    ('hazard', TEX + '红白相间的 45 度斜条纹警示油漆，涂在金属板上，条纹宽度均匀，轻微磨损'),
    ('metal', TEX + '浅灰色拉丝不锈钢金属板，细密的横向拉丝，少量铆钉'),
    ('facade_a', FACADE + '暖米黄色外墙的六层公寓楼，整齐的白框窗户，小阳台上有绿植和晾晒的衣服', '1440x2560'),
    ('facade_b', FACADE + '红砖外墙的五层老式公寓，白色窗框，黑色铁艺消防梯', '1440x2560'),
    ('facade_c', FACADE + '现代写字楼，蓝色反光玻璃幕墙，银色竖向金属窗框', '1440x2560'),
    ('facade_d', FACADE + '薄荷绿和白色的粉彩外墙六层楼，木质百叶窗，花箱里开着红花', '1440x2560'),
    ('train_side', FLAT + '现代城市地铁列车一节车厢的整个侧面，从车头到车尾填满画面，白色车身，宝蓝色车窗带，一条橙色腰线，'
                   '两扇双开车门，一排车窗，车厢底部是黑色转向架和车轮', '3840x960'),
    ('train_front', FLAT + '现代城市地铁列车的车头正面，画面被车头完全填满，白色车身，宝蓝色大块挡风玻璃，两盏明亮的圆形车灯，橙色装饰条'),
    ('coin', '一枚金币的正面，正上方俯视，圆形金币恰好填满整个正方形画面、边缘贴着画面边框，中间浮雕一颗五角星，厚实的金色边缘，闪亮，高光，3D 手游道具'),
    ('sky', '城市远景天际线，晴朗的下午，上半部分是湛蓝渐变的天空和几朵白云，下半部分是远处密集的高楼剪影，带一点蓝色空气透视雾气，'
            '地平线在画面下方四分之一处，宽银幕全景，风格化写实手游背景，没有文字', '3840x960'),
    ('cover', '手机跑酷游戏的标题页，竖屏，一个戴棒球帽、穿蓝色连帽衫背着橙色背包的少年在城市地铁轨道上全速奔跑，动态模糊，'
              '身后一列白蓝相间的地铁迎面驶来车灯耀眼，空中飘着一串金币，两边是彩色的城市楼房，夸张的广角透视，电影感光影，'
              '画面上方三分之一留出干净的天空给标题，3D 卡通渲染，色彩明快饱和，高品质手游宣传图，不要任何文字', '1440x2560'),
]
def load(aid):
    return Image.open(os.path.join(RAW, aid + '.jpg')).convert('RGB')
def have(aid):
    return os.path.isfile(os.path.join(RAW, aid + '.jpg'))
def save(im, name, q=85):
    im.save(os.path.join(OUT, name), quality=q, optimize=True)
def trim(im, frac=0.6):
    """裁掉画面边上的背景：模型常在楼 / 车厢四周留一圈白底或天空（提示词说了填满也不听）。
    背景色 = 四条边的中位色；一列（行）里「离背景色远」的像素过 frac 才算主体，取主体列（行）的范围"""
    a = np.asarray(im).astype(np.float32)
    border = np.concatenate([a[:4].reshape(-1, 3), a[-4:].reshape(-1, 3), a[:, :4].reshape(-1, 3), a[:, -4:].reshape(-1, 3)])
    fg = np.sqrt(((a - np.median(border, 0)) ** 2).sum(2)) > 45
    cols = np.where(fg.mean(0) > frac)[0]; rows = np.where(fg.mean(1) > frac)[0]
    if len(cols) < a.shape[1] * 0.5 or len(rows) < a.shape[0] * 0.5:
        return im
    return im.crop((int(cols[0]), int(rows[0]), int(cols[-1]) + 1, int(rows[-1]) + 1))
def process():
    os.makedirs(OUT, exist_ok=True)
    man = {'generatedBy': isl.MODEL, 'files': []}
    for aid in ('ballast', 'sleeper', 'concrete', 'hazard', 'metal'):
        if have(aid): save(isl.seamless(load(aid), 512), aid + '.jpg'); man['files'].append(aid)
    for aid in ('facade_a', 'facade_b', 'facade_c', 'facade_d'):
        if have(aid): save(trim(load(aid)).resize((512, 1024), Image.LANCZOS), aid + '.jpg'); man['files'].append(aid)
    if have('train_side'): save(trim(load('train_side'), 0.5).resize((1024, 256), Image.LANCZOS), 'train_side.jpg', 88); man['files'].append('train_side')
    if have('train_front'): save(trim(load('train_front')).resize((512, 512), Image.LANCZOS), 'train_front.jpg'); man['files'].append('train_front')
    if have('coin'): save(load('coin').resize((256, 256), Image.LANCZOS), 'coin.jpg', 88); man['files'].append('coin')
    if have('sky'):
        sky = load('sky').resize((1024, 512), Image.LANCZOS)    # 2:1 的 2 的幂；横向拉伸被背板的宽高比补回来
        save(sky, 'sky.jpg'); man['files'].append('sky')
        a = np.asarray(sky).astype(np.float32)
        man['fog'] = '#%02x%02x%02x' % tuple(int(v) for v in a[int(512 * 0.70):int(512 * 0.80)].mean((0, 1)))
        man['skyTop'] = '#%02x%02x%02x' % tuple(int(v) for v in a[:12].mean((0, 1)))
    if have('cover'):
        cv = load('cover').resize((720, 1280), Image.LANCZOS)
        save(cv, 'cover.jpg', 82); man['files'].append('cover')
        ic = cv.crop((60, 420, 660, 1020)).resize((144, 144), Image.LANCZOS).convert('RGBA')
        mask = Image.new('L', (576, 576), 0)
        ImageDraw.Draw(mask).rounded_rectangle((0, 0, 575, 575), radius=128, fill=255)
        ic.putalpha(mask.resize((144, 144), Image.LANCZOS))
        ic.save(os.path.join(ROOT, 'sample-game', 'play-3d-runner', 'icon.png'), optimize=True)
    json.dump(man, open(os.path.join(OUT, 'manifest.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    sizes = {f: os.path.getsize(os.path.join(OUT, f)) for f in os.listdir(OUT)}
    print('产物：' + '，'.join(f'{k} {v // 1024}KB' for k, v in sorted(sizes.items())) + f'，共 {sum(sizes.values()) // 1024}KB')
    missing = [a[0] for a in ASSETS if not have(a[0])]
    if missing: print('⚠ 还缺：' + ', '.join(missing))
def main():
    os.makedirs(RAW, exist_ok=True)
    args = sys.argv[1:]
    if '--process-only' not in args:
        if not os.environ.get('ARK_API_KEY'): sys.exit('要设 ARK_API_KEY（火山方舟的 API Key）')
        redo = set(args[args.index('--redo') + 1].split(',')) if '--redo' in args else set()
        todo = [a for a in ASSETS if a[0] in redo or not have(a[0])]
        print(f'要生成 {len(todo)} 张（已缓存 {len(ASSETS) - len(todo)} 张）', flush=True)
        with cf.ThreadPoolExecutor(max_workers=5) as ex:
            list(ex.map(lambda a: isl.generate(a[0], a[1], a[2] if len(a) > 2 else '2K'), todo))
    process()
if __name__ == '__main__':
    main()
