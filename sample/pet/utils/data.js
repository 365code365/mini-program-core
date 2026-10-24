// 萌宠家：离线演示数据
const pets = [
  { id: 'p1', name: '团子', kind: '猫', breed: '英国短毛猫 · 蓝猫', gender: '公', neutered: true, birth: '2022-05-18', age: '2 岁 5 个月', weight: 4.6, chip: '900 1180 0035 6721', color: 'linear-gradient(135deg, #8fb3d9, #5b7fae)',
    records: [
      { type: '疫苗', name: '猫三联 第 3 针', date: '2023-10-12', next: '2024-10-12', status: 'due' },
      { type: '疫苗', name: '狂犬疫苗', date: '2023-10-26', next: '2024-10-26', status: 'soon' },
      { type: '驱虫', name: '体内驱虫 · 拜宠清', date: '2024-08-02', next: '2024-11-02', status: 'ok' },
      { type: '驱虫', name: '体外驱虫 · 大宠爱', date: '2024-09-15', next: '2024-10-15', status: 'due' },
      { type: '体检', name: '年度体检 · 血常规', date: '2024-06-20', next: '2025-06-20', status: 'ok' }
    ],
    weights: [3.9, 4.1, 4.3, 4.4, 4.6, 4.6] },
  { id: 'p2', name: '可乐', kind: '狗', breed: '威尔士柯基 · 三色', gender: '母', neutered: false, birth: '2021-03-02', age: '3 岁 7 个月', weight: 11.2, chip: '900 1180 0041 2209', color: 'linear-gradient(135deg, #ffc58a, #f08a3c)',
    records: [
      { type: '疫苗', name: '犬八联', date: '2024-03-08', next: '2025-03-08', status: 'ok' },
      { type: '疫苗', name: '狂犬疫苗', date: '2024-03-08', next: '2025-03-08', status: 'ok' },
      { type: '驱虫', name: '内外同驱 · 超可信', date: '2024-09-28', next: '2024-10-28', status: 'soon' },
      { type: '体检', name: '牙齿检查 · 洁牙', date: '2024-05-11', next: '2024-11-11', status: 'ok' }
    ],
    weights: [10.4, 10.8, 11.0, 11.5, 11.3, 11.2] }
];
pets.forEach(p => { p.sx = p.name.charAt(0); });

const services = [
  { id: 's1', name: '基础洗护', desc: '洗澡吹干 · 耳道清洁 · 剪指甲', price: 88, min: 60 },
  { id: 's2', name: '精致美容造型', desc: '洗护 + 全身修剪造型', price: 198, min: 120 },
  { id: 's3', name: 'SPA 药浴', desc: '舒缓皮肤 · 去屑止痒', price: 128, min: 40 },
  { id: 's4', name: '体内外驱虫', desc: '含药费 · 按体重给药', price: 65, min: 15 },
  { id: 's5', name: '洁齿护理', desc: '超声波洁牙（非麻醉）', price: 158, min: 30 }
];

const stores = [
  { id: 'st1', name: '萌宠家 · 西溪银泰店', dist: '0.9km', rate: 4.9, addr: '余杭区五常大道 1 号西溪银泰城 B1', tags: ['洗护', '寄养', '医疗'], open: '09:00-21:00', color: 'linear-gradient(135deg, #ffd29a, #ff9a5a)' },
  { id: 'st2', name: '萌宠家 · 文三路店', dist: '2.4km', rate: 4.8, addr: '西湖区文三路 259 号昌地火炬大厦 1F', tags: ['洗护', '美容'], open: '10:00-20:30', color: 'linear-gradient(135deg, #a8e6cf, #4cbf9a)' },
  { id: 'st3', name: '萌宠家宠物医院 · 古荡店', dist: '3.1km', rate: 4.9, addr: '西湖区古墩路 83 号浙商财富中心 2F', tags: ['医疗', '疫苗', '体检'], open: '24 小时', color: 'linear-gradient(135deg, #bde4ff, #4aa8f0)' }
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

function findGoods(id) { return goods.find(g => g.id === id); }
function findPet(id) { return pets.find(p => p.id === id); }

module.exports = { pets, services, stores, goods, findGoods, findPet };
