// 萌宠家：离线演示数据（默认宠物、服务、门店、地区、商品）
// 默认宠物的健康记录写成「多少天前做的 + 周期（天）」，store 打开时按今天折算日期与到期状态：
// 写死日期的话，隔一年再打开就全成了「已到期」
const COLORS = [
  'linear-gradient(135deg, #8fb3d9, #5b7fae)',
  'linear-gradient(135deg, #ffc58a, #f08a3c)',
  'linear-gradient(135deg, #a8e6cf, #4cbf9a)',
  'linear-gradient(135deg, #f7a8c4, #e2648f)',
  'linear-gradient(135deg, #c3b1ff, #8a6cf0)'
];
const pets = [
  { id: 'p1', name: '团子', kind: '猫', breed: '英国短毛猫 · 蓝猫', gender: '公', neutered: true, birth: '2022-05-18', weight: 4.6, chip: '900 1180 0035 6721', color: COLORS[0],
    records: [
      { type: '疫苗', name: '猫三联 第 3 针', ago: 372, cycle: 365 },
      { type: '疫苗', name: '狂犬疫苗', ago: 352, cycle: 365 },
      { type: '驱虫', name: '体内驱虫 · 拜宠清', ago: 30, cycle: 90 },
      { type: '驱虫', name: '体外驱虫 · 大宠爱', ago: 33, cycle: 30 },
      { type: '体检', name: '年度体检 · 血常规', ago: 135, cycle: 365 }
    ],
    weights: [3.9, 4.1, 4.3, 4.4, 4.6, 4.6] },
  { id: 'p2', name: '可乐', kind: '狗', breed: '威尔士柯基 · 三色', gender: '母', neutered: false, birth: '2021-03-02', weight: 11.2, chip: '900 1180 0041 2209', color: COLORS[1],
    records: [
      { type: '疫苗', name: '犬八联', ago: 240, cycle: 365 },
      { type: '疫苗', name: '狂犬疫苗', ago: 240, cycle: 365 },
      { type: '驱虫', name: '内外同驱 · 超可信', ago: 20, cycle: 30 },
      { type: '体检', name: '牙齿检查 · 洁牙', ago: 175, cycle: 180 }
    ],
    weights: [10.4, 10.8, 11.0, 11.5, 11.3, 11.2] }
];
// 服务按类别分：一次预约只约一类（洗护和看病不在一个台子上做）；record：做完自动更新宠物档案里的这类记录
const cats = [
  { key: 'care', name: '洗护美容' },
  { key: 'med', name: '医疗保健' },
  { key: 'stay', name: '寄养' }
];
const services = [
  { id: 's1', cat: 'care', name: '基础洗护', desc: '洗澡吹干 · 耳道清洁 · 剪指甲', price: 88, min: 60 },
  { id: 's2', cat: 'care', name: '精致美容造型', desc: '洗护 + 全身修剪造型', price: 198, min: 120 },
  { id: 's3', cat: 'care', name: 'SPA 药浴', desc: '舒缓皮肤 · 去屑止痒', price: 128, min: 40 },
  { id: 's5', cat: 'care', name: '洁齿护理', desc: '超声波洁牙（非麻醉）', price: 158, min: 30 },
  { id: 's6', cat: 'med', name: '疫苗接种', desc: '猫三联 / 犬八联 / 狂犬疫苗，含接种费', price: 120, min: 20, record: '疫苗' },
  { id: 's4', cat: 'med', name: '体内外驱虫', desc: '含药费 · 按体重给药', price: 65, min: 15, record: '驱虫' },
  { id: 's7', cat: 'med', name: '健康体检', desc: '体格检查 + 血常规 + 粪检', price: 199, min: 40, record: '体检' },
  { id: 's8', cat: 'med', name: '门诊问诊', desc: '执业兽医面诊（药费、检查费另计）', price: 30, min: 30 },
  { id: 'stay', cat: 'stay', name: '寄养 · 独立房', desc: '每日 2 次视频回传 · 含猫砂 / 尿垫', price: 89, min: 0, perNight: true }
];
// cats：门店能做哪几类服务；start / end：可预约的到店时间
const stores = [
  { id: 'st1', name: '萌宠家 · 西溪银泰店', rate: 4.9, addr: '余杭区五常大道 1 号西溪银泰城 B1', tags: ['洗护', '寄养', '医疗'], cats: ['care', 'stay', 'med'],
    open: '09:00-21:00', start: '09:00', end: '20:00', phone: '0571-8826 1031', lat: 30.2741, lng: 120.0617, color: 'linear-gradient(135deg, #ffd29a, #ff9a5a)' },
  { id: 'st2', name: '萌宠家 · 文三路店', rate: 4.8, addr: '西湖区文三路 259 号昌地火炬大厦 1F', tags: ['洗护', '美容'], cats: ['care'],
    open: '10:00-20:30', start: '10:00', end: '19:30', phone: '0571-8826 1032', lat: 30.2788, lng: 120.1301, color: 'linear-gradient(135deg, #a8e6cf, #4cbf9a)' },
  { id: 'st3', name: '萌宠家宠物医院 · 古荡店', rate: 4.9, addr: '西湖区古墩路 83 号浙商财富中心 2F', tags: ['医疗', '疫苗', '体检'], cats: ['med'],
    open: '24 小时', start: '00:00', end: '23:30', phone: '0571-8822 6120', lat: 30.2869, lng: 120.1082, color: 'linear-gradient(135deg, #bde4ff, #4aa8f0)' }
];
const EMERGENCY = '0571-8822 6120';
// 首页左上角的地区：换了地区，门店距离与排序跟着变
const areas = [
  { name: '杭州 · 西溪', dist: { st1: 0.9, st2: 2.4, st3: 3.1 } },
  { name: '杭州 · 文三路', dist: { st1: 3.6, st2: 0.5, st3: 1.8 } },
  { name: '杭州 · 古荡', dist: { st1: 2.9, st2: 1.6, st3: 0.4 } }
];
const goods = [
  { id: 'g1', cat: '主粮', name: '鲜肉冻干全价猫粮 1.5kg', sub: '78% 动物蛋白 · 无谷配方', price: 139, orig: 169, sales: '2.1 万', img: '/images/goods/g1.jpg', specs: ['1.5kg', '3kg', '6kg'] },
  { id: 'g2', cat: '零食', name: '鸡肉蛋黄主食罐 85g×12', sub: '整肉可见 · 补水好帮手', price: 69, orig: 89, sales: '8,640', img: '/images/goods/g2.jpg', specs: ['12 罐', '24 罐'] },
  { id: 'g3', cat: '玩具', name: '羽毛逗猫棒 + 铃铛球套装', sub: '可替换羽毛 · 耐咬不掉毛', price: 29.9, orig: 39, sales: '1.3 万', img: '/images/goods/g3.jpg', specs: ['套装 A', '套装 B'] },
  { id: 'g4', cat: '用品', name: '不锈钢防滑双碗 斜口护颈', sub: '15° 斜口 · 可拆洗', price: 49, orig: 59, sales: '5,210', img: '/images/goods/g4.jpg', specs: ['小号', '中号', '大号'] },
  { id: 'g5', cat: '零食', name: '洁齿磨牙骨 牛奶味 20 支', sub: '减少牙垢 · 清新口气', price: 39, orig: 49, sales: '9,870', img: '/images/goods/g5.jpg', specs: ['小型犬', '中大型犬'] },
  { id: 'g6', cat: '洗护', name: '氨基酸温和沐浴露 500ml', sub: '弱酸性 · 猫狗通用', price: 59, orig: 79, sales: '6,330', img: '/images/goods/g6.jpg', specs: ['500ml', '1L'] },
  { id: 'g7', cat: '用品', name: '四季通用可拆洗宠物窝', sub: '加厚棉垫 · 防滑底', price: 129, orig: 169, sales: '3,920', img: '/images/goods/g7.jpg', specs: ['S', 'M', 'L'] },
  { id: 'g8', cat: '零食', name: '风干鸭肉条 营养肉干 200g', sub: '单一肉源 · 0 添加', price: 35, orig: 45, sales: '1.6 万', img: '/images/goods/g8.jpg', specs: ['200g', '500g'] }
];
const SERVICE_PHONE = '400-826-2031';
function findGoods(id) { return goods.find(g => g.id === id); }
function findService(id) { return services.find(s => s.id === id); }
function findStore(id) { return stores.find(s => s.id === id) || stores[0]; }
module.exports = { COLORS, pets, cats, services, stores, areas, goods, EMERGENCY, SERVICE_PHONE, findGoods, findService, findStore };
