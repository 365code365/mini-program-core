# 生成 iOS 随包字体：思源黑体（可变，取指定字重）做汉字 + Inter（可变）做西文 / 数字 / 西文标点，
# 度量照苹方 SC。对标的是微信 iOS（苹方 + SF），对照基准见 target/ios_ref（WebKit 渲染同一页面）。
#
#   python tools/ios-fonts/bake.py <思源 VF> <Inter VF> <思源字重> <Inter 字重> <输出.ttf> <PostScript 名>
#   （依赖 fontTools；随包的两份：380 400 MiniRenderSans-Regular、580 600 MiniRenderSans-Bold，见同目录 README.md）
#
# 为什么这样拼：
# - 思源默认字重偏黑（Regular 每字墨量比苹方 Regular 多约 4%，Bold 比苹方 Semibold 多 13%），按墨量对到苹方：
#   常规 380、粗体 580；
# - 思源的西文是 Source Sans：数字等宽且窄（0.553em，苹方 0.600 起），空格 0.223em（苹方 0.333），
#   字形也和 SF 差得远 —— 「Rust」「2048」「¥129.90」一眼就不像 iOS。Inter 与 SF 同类（大 x 高、开放字怀），
#   缩 0.98 后大写高 714、x 高 535，苹方是 714 / 517；
# - 思源的汉字比苹方整体高约 0.02em（「国」顶 793 / 底 -80，苹方 771 / -98）：整体下移 18 单位，基线上的位置对齐；
# - 行度量（hhea / OS/2）照苹方：引擎与 WebKit 都按它们定行盒。
# 输出用 TrueType（glyf，二次曲线）：引擎的光栅化器读 glyf 最稳，也不需要可变字体支持。
import sys
from fontTools.ttLib import TTFont, newTable
from fontTools.varLib import instancer
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.recordingPen import DecomposingRecordingPen

shs_src, inter_src, shs_w, inter_w, out, ps = sys.argv[1], sys.argv[2], float(sys.argv[3]), float(sys.argv[4]), sys.argv[5], sys.argv[6]
CJK_SHIFT = -18          # 汉字整体下移（单位 / 1000em）
LATIN_SCALE = 0.98       # Inter → 苹方西文的大小
# 用 Inter 的码位：基本拉丁、拉丁-1 补充与扩展 A、通用标点（弯引号、破折号、省略号…）、货币符号、几个常用符号
LATIN = set(range(0x20, 0x7F)) | set(range(0xA0, 0x250)) | set(range(0x2010, 0x2028)) | set(range(0x2030, 0x205F)) \
    | set(range(0x20A0, 0x20C1)) | {0x2122, 0x2190, 0x2191, 0x2192, 0x2193, 0x2212, 0x00D7, 0x00F7}
# 中文排版里这些是全角的（苹方也是全角）：留给思源
KEEP_CJK = {0x00B7, 0x2014, 0x2015, 0x2026, 0x2018, 0x2019, 0x201C, 0x201D}

shs = instancer.instantiateVariableFont(TTFont(shs_src), {'wght': shs_w}, updateFontNames=False)
inter = instancer.instantiateVariableFont(TTFont(inter_src), {'wght': inter_w, 'opsz': 14}, updateFontNames=False)
s_upm = shs['head'].unitsPerEm
i_upm = inter['head'].unitsPerEm
k = LATIN_SCALE * s_upm / i_upm

order = list(shs.getGlyphOrder())
gs = shs.getGlyphSet()
glyf = newTable('glyf'); glyf.glyphOrder = order; glyf.glyphs = {}
hmtx = dict(shs['hmtx'].metrics)
vmtx = dict(shs['vmtx'].metrics) if 'vmtx' in shs else None


def quad(glyph, glyphset, transform):
    # 组件字形（Inter 的 é = e + ´）先拆成轮廓：拼进来之后原来的组件名在这个字体里不存在
    rec = DecomposingRecordingPen(glyphset)
    glyph.draw(rec)
    pen = TTGlyphPen(None)
    rec.replay(Cu2QuPen(TransformPen(pen, transform), max_err=0.5, reverse_direction=True))
    return pen.glyph()


for name in order:
    glyf.glyphs[name] = quad(gs[name], gs, (1, 0, 0, 1, 0, CJK_SHIFT))

# 西文换成 Inter：新建字形，cmap 指过去
igs = inter.getGlyphSet()
icm = inter.getBestCmap()
cmap_tables = [t for t in shs['cmap'].tables if t.isUnicode()]
replaced = 0
for cp in sorted(LATIN - KEEP_CJK):
    if cp not in icm:
        continue
    src = icm[cp]
    name = f'lat.{cp:04X}'
    order.append(name)
    glyf.glyphs[name] = quad(igs[src], igs, (k, 0, 0, k, 0, 0))
    adv = round(inter['hmtx'][src][0] * k)
    g = glyf.glyphs[name]
    g.recalcBounds(glyf)
    lsb = getattr(g, 'xMin', 0)
    hmtx[name] = (adv, lsb)
    if vmtx is not None:
        vmtx[name] = (s_upm, 0)
    for t in cmap_tables:
        if t.format in (4, 12) and (cp <= 0xFFFF or t.format == 12):
            t.cmap[cp] = name
    replaced += 1

glyf.glyphOrder = order
shs.setGlyphOrder(order)
shs['glyf'] = glyf
shs['loca'] = newTable('loca')
shs['hmtx'].metrics = hmtx
if vmtx is not None:
    shs['vmtx'].metrics = vmtx
shs['head'].indexToLocFormat = 1
shs['head'].glyphDataFormat = 0
maxp = shs['maxp']; maxp.tableVersion = 0x00010000; maxp.numGlyphs = len(order)
for a in ('maxPoints', 'maxContours', 'maxCompositePoints', 'maxCompositeContours', 'maxZones', 'maxTwilightPoints',
          'maxStorage', 'maxFunctionDefs', 'maxInstructionDefs', 'maxStackElements', 'maxSizeOfInstructions',
          'maxComponentElements', 'maxComponentDepth'):
    setattr(maxp, a, 1 if a == 'maxZones' else 0)
# 西文换了字形，原来的排版表（连字、字距、竖排替换）里还指着思源的西文：去掉，免得替换回去
for t in ('CFF ', 'CFF2', 'VORG', 'GPOS', 'GSUB', 'BASE', 'GDEF', 'STAT', 'kern'):
    if t in shs:
        del shs[t]
shs['post'].formatType = 3.0
shs.sfntVersion = '\x00\x01\x00\x00'
h = shs['hhea']; h.ascent, h.descent, h.lineGap = 1060, -340, 0
o = shs['OS/2']; o.usWinAscent, o.usWinDescent = 1060, 340
o.sTypoAscender, o.sTypoDescender, o.sTypoLineGap = 860, -140, 400
o.usWeightClass = int(shs_w)
o.fsType = 0
# 改名：两份字体都是 SIL OFL 1.1，「Source」是思源的保留字体名，改过的版本不能再叫它
style = ps.split('-')[-1]
names = {1: 'MiniRender Sans', 2: style, 3: ps, 4: f'MiniRender Sans {style}', 6: ps, 16: 'MiniRender Sans', 17: style}
nt = shs['name']
nt.names = [r for r in nt.names if r.nameID not in names and r.nameID not in (18, 21, 22)]
for nid, text in names.items():
    nt.setName(text, nid, 3, 1, 0x409)
shs.save(out)
print(out, len(order), 'glyphs, latin replaced', replaced)
