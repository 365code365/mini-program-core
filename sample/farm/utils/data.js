// 田园直供的商品 / 产地 / 直播数据（真实产区、按 2026 年产地直发的常见行情定价）。
// sku 的键是「各维度选项下标用 | 连起来」，stock 为 0 的组合在规格面板里置灰。
const G = '/images/goods/';

function sku(dims, table) {
  // table: { '0|0': [单买价, 拼团价, 库存], ... }
  const out = {};
  Object.keys(table).forEach((k) => {
    const [price, groupPrice, stock] = table[k];
    out[k] = { price, groupPrice, stock };
  });
  return { dims, map: out };
}

const goods = [
  {
    id: 'g01', name: '赣南脐橙 现摘现发', origin: '江西赣州·信丰', farmer: 'f1', cover: G + 'orange.jpg',
    imgs: [G + 'orange.jpg', '/images/story/s1.jpg'], price: 29.9, groupPrice: 22.9, groupSize: 3, sold: 12860, season: '11月—次年2月',
    tags: ['坏果包赔', '48h发货'], unit: '箱',
    desc: '信丰县安西镇果园直采，树上留果到 11 月中旬才摘，糖度 12°以上，果皮薄、无籽多汁。',
    sku: sku([{ name: '重量', opts: ['5斤装', '10斤装'] }, { name: '果径', opts: ['65-70mm', '70-80mm', '80mm以上'] }], {
      '0|0': [29.9, 22.9, 320], '0|1': [35.9, 27.9, 210], '0|2': [42.9, 34.9, 60],
      '1|0': [52.9, 42.9, 180], '1|1': [62.9, 52.9, 90], '1|2': [76.9, 66.9, 0]
    })
  },
  {
    id: 'g02', name: '阿克苏冰糖心苹果', origin: '新疆阿克苏·红旗坡', farmer: 'f2', cover: G + 'apple.jpg',
    imgs: [G + 'apple.jpg', '/images/story/s2.jpg'], price: 39.9, groupPrice: 32.8, groupSize: 2, sold: 9420, season: '10月—次年3月',
    tags: ['糖心率90%', '顺丰冷链'], unit: '箱',
    desc: '红旗坡农场海拔 1100 米，昼夜温差大，果核周围自然形成透明糖心，脆甜不面。',
    sku: sku([{ name: '重量', opts: ['5斤装', '10斤装'] }, { name: '规格', opts: ['中果 75mm', '大果 85mm'] }], {
      '0|0': [39.9, 32.8, 260], '0|1': [49.9, 41.8, 120], '1|0': [69.9, 59.8, 80], '1|1': [86.9, 76.8, 0]
    })
  },
  {
    id: 'g03', name: '五常稻花香大米 当季新米', origin: '黑龙江五常·民乐乡', farmer: 'f3', cover: G + 'rice.jpg',
    imgs: [G + 'rice.jpg', '/images/story/s2.jpg'], price: 69, groupPrice: 59, groupSize: 3, sold: 20431, season: '10月新米上市',
    tags: ['溯源码', '真空包装'], unit: '袋',
    desc: '稻花香 2 号，一年一季，生长期 145 天。真空小包装，开袋即是清香。',
    sku: sku([{ name: '规格', opts: ['5kg', '10kg'] }], { '0': [69, 59, 500], '1': [128, 112, 260] })
  },
  {
    id: 'g04', name: '恩施散养土鸡蛋', origin: '湖北恩施·利川', farmer: 'f2', cover: G + 'egg.jpg',
    imgs: [G + 'egg.jpg', '/images/story/s3.jpg'], price: 45.8, groupPrice: 38.8, groupSize: 2, sold: 7610, season: '全年',
    tags: ['破损包赔', '林下散养'], unit: '盒',
    desc: '山林散养 300 天以上的土鸡下的蛋，蛋黄橙红，每枚 45~55 克。',
    sku: sku([{ name: '数量', opts: ['30枚', '60枚'] }], { '0': [45.8, 38.8, 340], '1': [86.8, 74.8, 150] })
  },
  {
    id: 'g05', name: '秦岭百花土蜂蜜', origin: '陕西商洛·柞水', farmer: 'f3', cover: G + 'honey.jpg',
    imgs: [G + 'honey.jpg', '/images/story/s3.jpg'], price: 68, groupPrice: 56, groupSize: 3, sold: 5320, season: '每年一割',
    tags: ['波美度42+', '玻璃瓶装'], unit: '瓶',
    desc: '中华蜂一年只取一次蜜，自然成熟封盖，冬季低温会结晶，属正常现象。',
    sku: sku([{ name: '规格', opts: ['500g', '1000g'] }], { '0': [68, 56, 210], '1': [126, 108, 0] })
  },
  {
    id: 'g06', name: '云南高原蓝莓 Jumbo果', origin: '云南玉溪·澄江', farmer: 'f1', cover: G + 'berry.jpg',
    imgs: [G + 'berry.jpg', '/images/story/s1.jpg'], price: 59.9, groupPrice: 49.9, groupSize: 3, sold: 11208, season: '12月—次年5月',
    tags: ['18mm+大果', '冰袋保鲜'], unit: '箱',
    desc: '澄江高原日照足，果粉完整，单果直径 18mm 以上，空运直发。',
    sku: sku([{ name: '规格', opts: ['125g×4盒', '125g×8盒'] }], { '0': [59.9, 49.9, 180], '1': [109, 92, 70] })
  },
  {
    id: 'g07', name: '海南贵妃芒 树上熟', origin: '海南三亚·崖州', farmer: 'f2', cover: G + 'mango.jpg',
    imgs: [G + 'mango.jpg', '/images/story/s2.jpg'], price: 36.8, groupPrice: 29.9, groupSize: 2, sold: 8930, season: '1月—5月',
    tags: ['坏果包赔', '自然熟'], unit: '箱',
    desc: '红金龙贵妃芒，八成熟采摘，到手 2~3 天自然转黄即可食用。',
    sku: sku([{ name: '重量', opts: ['3斤装', '5斤装'] }, { name: '果型', opts: ['中果', '大果'] }], {
      '0|0': [24.8, 19.9, 200], '0|1': [29.8, 24.9, 0], '1|0': [36.8, 29.9, 260], '1|1': [44.8, 37.9, 90]
    })
  },
  {
    id: 'g08', name: '宁夏中宁头茬枸杞', origin: '宁夏中卫·中宁', farmer: 'f3', cover: G + 'goji.jpg',
    imgs: [G + 'goji.jpg', '/images/story/s3.jpg'], price: 38, groupPrice: 32, groupSize: 2, sold: 6402, season: '6月头茬',
    tags: ['免洗即食', '无硫熏'], unit: '罐',
    desc: '6 月采摘的头茬果，粒大肉厚，280 粒 / 50 克，可直接干嚼或泡水。',
    sku: sku([{ name: '规格', opts: ['250g罐装', '500g袋装'] }], { '0': [38, 32, 300], '1': [68, 58, 160] })
  },
  {
    id: 'g09', name: '丹东99草莓', origin: '辽宁丹东·东港', farmer: 'f1', cover: G + 'strawberry.jpg',
    imgs: [G + 'strawberry.jpg', '/images/story/s1.jpg'], price: 89, groupPrice: 79, groupSize: 3, sold: 4205, season: '12月—次年4月',
    tags: ['单果25g+', '泡沫箱+冰袋'], unit: '箱',
    desc: '东港红颜 99 草莓，冬季大棚慢长 50 天，香气浓、汁水足。',
    sku: sku([{ name: '规格', opts: ['3斤 大果', '3斤 特大果'] }], { '0': [89, 79, 90], '1': [119, 106, 30] })
  },
  {
    id: 'g10', name: '明前龙井 一级', origin: '浙江杭州·梅家坞', farmer: 'f2', cover: G + 'tea.jpg',
    imgs: [G + 'tea.jpg', '/images/story/s2.jpg'], price: 168, groupPrice: 148, groupSize: 2, sold: 1830, season: '清明前采',
    tags: ['当年新茶', '铁罐密封'], unit: '罐',
    desc: '龙井 43 号，一芽一叶初展，手工炒制，豆花香明显。',
    sku: sku([{ name: '规格', opts: ['100g', '250g'] }], { '0': [168, 148, 120], '1': [398, 358, 40] })
  },
  {
    id: 'g11', name: '东北甜糯玉米 真空装', origin: '吉林白城·洮南', farmer: 'f3', cover: G + 'corn.jpg',
    imgs: [G + 'corn.jpg', '/images/story/s3.jpg'], price: 32.9, groupPrice: 26.9, groupSize: 3, sold: 15602, season: '全年',
    tags: ['开袋即食', '非转基因'], unit: '箱',
    desc: '黄白双色甜糯玉米，鲜穗当天蒸煮后真空锁鲜，常温保存 9 个月。',
    sku: sku([{ name: '规格', opts: ['10穗', '20穗'] }], { '0': [32.9, 26.9, 400], '1': [59.9, 49.9, 220] })
  },
  {
    id: 'g12', name: '阳澄湖大闸蟹 现捕现发', origin: '江苏苏州·阳澄湖', farmer: 'f1', cover: G + 'crab.jpg',
    imgs: [G + 'crab.jpg', '/images/story/s2.jpg'], price: 398, groupPrice: 358, groupSize: 2, sold: 2130, season: '9月—12月',
    tags: ['死蟹包赔', '顺丰次日达'], unit: '盒',
    desc: '湖区围网养殖，公蟹 4 两、母蟹 3 两，每只带防伪蟹扣。',
    sku: sku([{ name: '规格', opts: ['4对 公4两母3两', '5对 公4.5两母3.5两'] }], { '0': [398, 358, 60], '1': [568, 518, 0] })
  }
];

const farmers = {
  f1: { id: 'f1', name: '刘建国', title: '信丰脐橙合作社 · 种果 26 年', avatar: '/images/misc/farmer1.jpg', phone: '0797-3321578' },
  f2: { id: 'f2', name: '阿依古丽', title: '红旗坡果园 · 第二代果农', avatar: '/images/misc/farmer2.jpg', phone: '0997-2135566' },
  f3: { id: 'f3', name: '王守田', title: '民乐乡水稻种植大户', avatar: '/images/misc/farmer3.jpg', phone: '0451-57324410' }
};

const stories = [
  {
    id: 's1', title: '信丰果园：等一树橙子红透', sub: '刘建国和他的 600 棵脐橙树', img: '/images/story/s1.jpg', farmer: 'f1', goods: ['g01', 'g06', 'g09'],
    body: '<p>信丰县安西镇的丘陵上，刘建国的果园已经种了 26 年。<b>赣南脐橙</b>讲究「留树保鲜」，别家 10 月就开摘，他要等到 11 月中旬果皮完全转红。</p>' +
      '<p>每天清晨 5 点采摘，中午前完成分拣、打蜡以外的全部工序 —— 我们<b>不打保鲜蜡</b>，所以果皮看起来没那么亮。</p>' +
      '<p>下午 4 点前装箱，顺丰从镇上集散点直接发往全国，<b>48 小时内送到</b>大多数城市。</p>'
  },
  {
    id: 's2', title: '五常稻田：一年只种一季', sub: '王守田的 145 天', img: '/images/story/s2.jpg', farmer: 'f3', goods: ['g03', 'g11'],
    body: '<p>五常市民乐乡位于拉林河畔，<b>稻花香 2 号</b>从插秧到收割要 145 天，一年只有一季。</p>' +
      '<p>王守田家的 80 亩稻田引山泉水灌溉，每块田都有溯源码，扫码能看到这袋米来自哪块地、哪天收割。</p>' +
      '<p>新米 10 月上市，<b>现磨现发</b>，收到时离出米不超过 7 天。</p>'
  },
  {
    id: 's3', title: '秦岭深处：中蜂一年一割', sub: '柞水县的老蜂箱', img: '/images/story/s3.jpg', farmer: 'f3', goods: ['g05', 'g04', 'g08'],
    body: '<p>柞水县海拔 1200 米的山坳里，散放着 300 多个老式木桶蜂箱。中华蜂采百花，<b>一年只取一次蜜</b>。</p>' +
      '<p>取蜜前要确认蜂蜜已经封盖成熟，波美度在 42 以上。冬天温度低时蜂蜜会结晶，这正是没有掺糖浆的证明。</p>'
  }
];

const live = {
  title: '赣南果园现场采摘 · 今日 22.9 元拼团', farmer: 'f1', viewers: 23416,
  src: 'https://live.tianyuan-direct.cn/live/xinfeng-orchard.flv',
  replay: '/images/misc/origin.mp4', cover: '/images/misc/live-cover.jpg',
  goods: ['g01', 'g06', 'g09', 'g04'],
  schedule: [
    { id: 'l1', time: '明天 10:00', title: '五常新米开仓直播', farmer: 'f3' },
    { id: 'l2', time: '周六 19:30', title: '阿克苏苹果糖心现场切', farmer: 'f2' },
    { id: 'l3', time: '周日 15:00', title: '秦岭取蜜直播', farmer: 'f3' }
  ],
  comments: [
    ['江南小满', '去年买过，橙子真的甜！'], ['老陈在杭州', '10 斤装还有大果吗'], ['豆豆妈', '已拼团，等发货~'],
    ['风起时', '果园好漂亮'], ['阿杰', '主播能切一个看看吗'], ['青柠', '冰糖心苹果什么时候开播'],
    ['一颗橙子', '坏果真的包赔吗'], ['Mia', '三人团还差一个，有人来吗']
  ]
};

const DELIVERY_SLOTS = ['不限时段', '上午 9:00-12:00', '下午 13:00-18:00', '晚上 18:00-21:00'];

function goodsById(id) {
  return goods.find((g) => g.id === id);
}

module.exports = { goods, farmers, stories, live, goodsById, DELIVERY_SLOTS };
