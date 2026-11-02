// 云川博物院的馆藏与展陈数据（虚构馆，器物类型、年代、尺寸按同类真实文物的常见值写）
const HALLS = [
  { id: 'h1', name: '青铜馆', floor: '一层', area: '1F 东区', cover: '/images/hall/h1.jpg', theme: '吉金铸礼',
    intro: '展出商周至汉代青铜礼器、兵器与铜镜 180 余件，按「礼制—生活—工艺」三个单元陈列，复原了一处西周贵族墓的列鼎组合。',
    exhibits: ['e1', 'e2'] },
  { id: 'h2', name: '陶瓷馆', floor: '一层', area: '1F 西区', cover: '/images/hall/h2.jpg', theme: '千年窑火',
    intro: '以云川本地窑口与宋元名窑为线索，陈列青瓷、白瓷、青花与彩瓷 260 余件，设有拉坯与烧成过程的互动展项。',
    exhibits: ['e3', 'e4'] },
  { id: 'h3', name: '书画馆', floor: '二层', area: '2F 北区', cover: '/images/hall/h3.jpg', theme: '翰墨丹青',
    intro: '书画因保护需要实行轮展，每季度更换一批。现展出宋元明清书画 64 件，展柜恒温 20℃、相对湿度 55%，照度不高于 50 勒克斯。',
    exhibits: ['e5', 'e6'] },
  { id: 'h4', name: '玉器馆', floor: '二层', area: '2F 南区', cover: '/images/hall/h4.jpg', theme: '温润如玉',
    intro: '从新石器时代的玉琮、玉璧到明清的陈设玉器，展出 150 余件，讲述「以玉事神」到「君子比德于玉」的观念变化。',
    exhibits: ['e7', 'e8'] },
  { id: 'h5', name: '民俗馆', floor: '三层', area: '3F 全层', cover: '/images/hall/h5.jpg', theme: '人间烟火',
    intro: '收录云川及周边地区的剪纸、刺绣、木版年画与节庆器具，周末有非遗传承人现场演示。',
    exhibits: ['e9', 'e10'] }
];

const EXHIBITS = {
  e1: { name: '兽面纹铜鼎', era: '西周早期', hall: 'h1', img: '/images/ex/e1.jpg', level: '一级文物', material: '青铜',
    size: '通高 42.5 厘米，口径 36.2 厘米', origin: '1978 年云川市青石岗 M2 出土',
    inscription: '唯八月初吉，伯作宝尊鼎，其万年子子孙孙永宝用。',
    desc: '<p>鼎为立耳、深腹、三柱足，口沿下饰一周<b>兽面纹带</b>，以云雷纹衬地，足上部饰牛首纹。</p><p>腹内壁铸铭文 3 行 18 字，记载器主「伯」为祭祀先祖作此鼎。青石岗 M2 共出土同形列鼎 5 件，是研究西周用鼎制度的重要实物。</p>' },
  e2: { name: '四神规矩纹铜镜', era: '东汉', hall: 'h1', img: '/images/ex/e2.jpg', level: '二级文物', material: '青铜',
    size: '直径 18.6 厘米，缘厚 0.5 厘米', origin: '1992 年云川市南郊砖室墓出土',
    inscription: '尚方作竟真大好，上有仙人不知老，渴饮玉泉饥食枣。',
    desc: '<p>圆钮、柿蒂纹钮座，座外方框内列十二地支。主纹区以<b>TLV 博局纹</b>分为四方八区，配置青龙、白虎、朱雀、玄武。</p><p>外区为一周铭文带与锯齿纹、云气纹缘，镜面至今仍可映出人影。</p>' },
  e3: { name: '龙泉窑青釉梅瓶', era: '南宋', hall: 'h2', img: '/images/ex/e3.jpg', level: '一级文物', material: '瓷',
    size: '高 31.2 厘米，口径 4.8 厘米，底径 9.6 厘米', origin: '2003 年云川市城南窖藏出土',
    inscription: '底部墨书「宝庆三年置」五字。',
    desc: '<p>小口、丰肩、敛腹，通体施<b>粉青釉</b>，釉层肥厚，莹润如玉，是龙泉窑鼎盛时期的典型器。</p><p>同一窖藏出土瓷器 37 件，底部墨书纪年为判定这批器物的年代提供了直接依据。</p>' },
  e4: { name: '青花缠枝莲纹梅瓶', era: '明永乐', hall: 'h2', img: '/images/ex/e4.jpg', level: '一级文物', material: '瓷',
    size: '高 34.8 厘米，口径 5.6 厘米，底径 11.5 厘米', origin: '1965 年旧藏，原藏云川府学',
    inscription: '无款。',
    desc: '<p>胎质细白，青花使用进口<b>苏麻离青</b>料，发色浓艳，晕散处可见铁锈斑。</p><p>腹部绘缠枝莲六朵，肩部饰垂云纹，近足处为变形莲瓣，布局疏密得当，是永乐青花的代表风格。</p>' },
  e5: { name: '《云川秋霁图》轴', era: '元', hall: 'h3', img: '/images/ex/e5.jpg', level: '一级文物', material: '纸本水墨',
    size: '纵 112.4 厘米，横 46.3 厘米', origin: '1956 年私人捐赠',
    inscription: '秋霁云川上，溪山入画图。至正七年冬日，写于松雪斋。',
    desc: '<p>画面自下而上分三段：近景坡石杂树、中景平湖渔舟、远景层峦叠嶂，以<b>披麻皴</b>写山石，笔墨清润。</p><p>右上自题诗一首并钤印两方，裱边有明清两代收藏印十一方，流传有序。本季度为最后一期展出，次年轮换休展。</p>' },
  e6: { name: '行书「山高水长」轴', era: '清', hall: 'h3', img: '/images/ex/e6.jpg', level: '三级文物', material: '纸本',
    size: '纵 132.0 厘米，横 33.5 厘米', origin: '1983 年征集',
    inscription: '山高水长。乾隆丙午仲春，书于云川官舍。',
    desc: '<p>四字一行直下，用笔沉着，结体<b>宽博</b>，取法颜真卿而参以行草笔意。</p><p>「山高水长」语出范仲淹《严先生祠堂记》，多用于称颂人的品德与声望。</p>' },
  e7: { name: '谷纹玉璧', era: '战国', hall: 'h4', img: '/images/ex/e7.jpg', level: '二级文物', material: '和田青白玉',
    size: '直径 16.3 厘米，孔径 4.1 厘米，厚 0.5 厘米', origin: '1987 年云川市北山楚墓出土',
    inscription: '无铭。',
    desc: '<p>玉质温润，局部有褐色沁。两面纹饰相同，内外缘各起<b>弦纹</b>一周，中间满布谷纹，排列整齐。</p><p>《周礼》载「以苍璧礼天」，玉璧是古代祭天的礼器，战国时也用作佩饰与馈赠。</p>' },
  e8: { name: '神人兽面纹玉琮', era: '新石器时代 良渚文化', hall: 'h4', img: '/images/ex/e8.jpg', level: '一级文物', material: '透闪石玉',
    size: '高 8.8 厘米，射径 17.1~17.6 厘米', origin: '2011 年云川市古城遗址出土',
    inscription: '无铭。',
    desc: '<p>外方内圆，四面以转角为中线，各刻一组<b>神人兽面纹</b>，细如发丝的阴线需借助放大镜才能看清。</p><p>玉琮是良渚文化最具代表性的礼器，被认为与沟通天地的宗教观念有关。</p>' },
  e9: { name: '团花剪纸「十二月花神」', era: '现代', hall: 'h5', img: '/images/ex/e9.jpg', level: '一般藏品', material: '红宣纸',
    size: '直径 48 厘米', origin: '2019 年云川剪纸传承人陈素梅捐赠',
    inscription: '正月梅花二月杏，三月桃花红似锦。',
    desc: '<p>以一张红宣纸对折十二次剪成，外圈十二个圆窗分别对应<b>十二月花神</b>，中心为八瓣宝相花。</p><p>云川剪纸 2014 年列入省级非物质文化遗产名录，每周六下午民俗馆有现场教学。</p>' },
  e10: { name: '苏绣牡丹团扇', era: '清晚期', hall: 'h5', img: '/images/ex/e10.jpg', level: '三级文物', material: '素绢、丝线、湘妃竹',
    size: '扇面直径 26 厘米，通长 44 厘米', origin: '2008 年征集',
    inscription: '扇柄刻「花开富贵」四字。',
    desc: '<p>扇面以<b>乱针绣</b>绣牡丹一枝，花瓣用十余种深浅不同的粉色丝线晕染过渡，叶片以施针表现正反。</p><p>扇框为湘妃竹，竹上天然斑点如泪痕，是清代江南闺阁常见的雅玩。</p>' }
};

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function ymd(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function addDays(d, n) { const x = new Date(d.getFullYear(), d.getMonth(), d.getDate()); x.setDate(x.getDate() + n); return x; }

// 特展的展期按「今天」前后排：保证任何一天打开都有进行中、即将开展两种状态
function expos() {
  const t = new Date();
  const mk = (id, title, sub, cover, place, s, e, ids, intro, price) => ({
    id, title, sub, cover, place, start: ymd(addDays(t, s)), end: ymd(addDays(t, e)), exhibits: ids, intro, price });
  return [
    mk('x1', '吉金万象', '云川青铜文明特展', '/images/expo/x1.jpg', '二层特展厅 A', -40, 66, ['e1', 'e2'],
      '<p>汇集省内 6 家博物馆的商周青铜器 126 件（组），其中一级文物 31 件。</p><p>展览分「<b>铸礼</b>」「<b>列鼎</b>」「<b>纹饰</b>」三个单元，首次完整展出青石岗墓地出土的五鼎四簋。</p>', '免费，需预约'),
    mk('x2', '青花流韵', '明清外销瓷精品展', '/images/expo/x2.jpg', '二层特展厅 B', -12, 98, ['e4', 'e3'],
      '<p>以海上丝绸之路为线索，展出明清外销青花瓷 210 件，其中 40 件为沉船出水文物。</p><p>展厅中央复原了一段 1:1 的福船货舱。</p>', '免费，需预约'),
    mk('x3', '溪山无尽', '宋元山水画特展', '/images/expo/x3.jpg', '书画馆专题厅', 9, 120, ['e5', 'e6'],
      '<p>集中展出宋元山水画真迹 28 件。为保护文物，每件作品展期不超过 45 天，分三期轮换。</p>', '成人 30 元，学生 15 元'),
    mk('x4', '锦绣华章', '江南刺绣艺术展', '/images/expo/x4.jpg', '三层临展厅', 25, 140, ['e10', 'e9'],
      '<p>从清代闺阁绣品到当代苏绣大师作品，展出 160 余件，现场有绣娘演示乱针绣与双面绣。</p>', '免费，需预约')
  ];
}

// 每周固定的社教活动（星期几 → 活动），周一闭馆
const WEEKLY = {
  2: [{ id: 'ev-tue', time: '10:00', title: '小小考古家：拼一件陶罐', place: '一层教育中心', quota: 20, tag: '亲子' }],
  3: [{ id: 'ev-wed', time: '14:30', title: '专家导览：青铜器上的饕餮纹', place: '青铜馆入口集合', quota: 30, tag: '导览' }],
  5: [{ id: 'ev-fri', time: '19:00', title: '夜场讲座：宋人如何画山水', place: '报告厅', quota: 120, tag: '讲座' }],
  6: [{ id: 'ev-sat1', time: '10:00', title: '非遗课堂：云川剪纸', place: '民俗馆工作坊', quota: 24, tag: '非遗' },
      { id: 'ev-sat2', time: '15:00', title: '手作体验：拓印铜镜纹样', place: '一层教育中心', quota: 20, tag: '亲子' }],
  0: [{ id: 'ev-sun', time: '14:00', title: '志愿者讲解：镇馆之宝十件', place: '一层大厅服务台', quota: 40, tag: '导览' }]
};

const SLOTS = [
  { id: 's1', label: '09:00-11:00', cap: 1200 },
  { id: 's2', label: '11:00-13:00', cap: 1200 },
  { id: 's3', label: '13:00-15:00', cap: 1500 },
  { id: 's4', label: '15:00-16:00', cap: 800 }
];

const NOTICE = [
  '开放时间 09:00-17:00，16:00 停止入馆；每周一闭馆（法定节假日除外）。',
  '实行实名预约，每人每日限约 1 次，每单最多 5 人；1.2 米以下儿童由成人陪同可免预约。',
  '入馆请出示预约凭证与本人身份证件，凭证当日当时段有效。',
  '展厅内禁止使用闪光灯、三脚架与自拍杆，书画馆禁止拍照。',
  '一层服务台提供免费寄存、轮椅与婴儿车借用。'
];

module.exports = { HALLS, EXHIBITS, expos, WEEKLY, SLOTS, NOTICE, ymd, addDays, pad };
