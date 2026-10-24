// 离线演示数据：附近商家与菜单（价格单位：元）
const shops = [
  {
    id: 's1', name: '老乡鸡（望京SOHO店）', logo: '鸡', c1: '#ffb347', c2: '#ff7a1a',
    cat: '中式快餐', rating: 4.8, sales: 3021, time: 28, distance: '1.2km',
    minOrder: 20, deliveryFee: 3, packFee: 1, tags: ['满30减8', '满50减15', '新客立减5'], brand: true,
    notice: '坚持现炒现做，肥西老母鸡汤每日现炖 6 小时'
  },
  {
    id: 's2', name: '和府捞面（来福士店）', logo: '面', c1: '#8fd3a4', c2: '#2e9e5b',
    cat: '面馆', rating: 4.7, sales: 1856, time: 32, distance: '1.8km',
    minOrder: 25, deliveryFee: 4, packFee: 2, tags: ['满40减10', '会员 95 折'], brand: true,
    notice: '书房里的养生面，草本骨汤熬足 12 小时'
  },
  {
    id: 's3', name: '西少爷肉夹馍', logo: '馍', c1: '#ffd36b', c2: '#f29a00',
    cat: '小吃', rating: 4.6, sales: 2410, time: 25, distance: '0.9km',
    minOrder: 15, deliveryFee: 2, packFee: 1, tags: ['满25减6', '第二份半价'], brand: false,
    notice: '腊汁肉每天凌晨 4 点开炖，馍现烤现夹'
  },
  {
    id: 's4', name: '鱼你在一起·酸菜鱼', logo: '鱼', c1: '#7cc6ff', c2: '#2a7fff',
    cat: '川湘菜', rating: 4.9, sales: 4120, time: 35, distance: '2.1km',
    minOrder: 30, deliveryFee: 5, packFee: 2, tags: ['满50减18', '满80减28'], brand: true,
    notice: '一人一份酸菜鱼，巴沙鱼片无刺嫩滑'
  },
  {
    id: 's5', name: '茶百道（望京街店）', logo: '茶', c1: '#ffa6c1', c2: '#e8487c',
    cat: '奶茶饮品', rating: 4.8, sales: 9860, time: 22, distance: '0.6km',
    minOrder: 18, deliveryFee: 2, packFee: 1, tags: ['满30减5', '杨枝甘露特价'], brand: true,
    notice: '鲜果现切，下单后 3 分钟出杯'
  },
  {
    id: 's6', name: '南城香（阜通店）', logo: '香', c1: '#ffb59a', c2: '#e2552d',
    cat: '中式快餐', rating: 4.5, sales: 1530, time: 30, distance: '1.5km',
    minOrder: 20, deliveryFee: 3, packFee: 1, tags: ['满35减9', '免费加饭'], brand: false,
    notice: '馄饨、烤串、米饭，北京人的家常味道'
  }
];

function dish(id, name, desc, price, sales, c1, c2, glyph, origin) {
  return { id, name, desc, price, sales, c1, c2, glyph, origin: origin || 0 };
}

// 每家店的菜单：按分类分组
const menus = {
  s1: [
    { name: '热销', items: [
      dish('d101', '农家小炒肉套餐', '精选前腿肉 + 杭椒，配米饭', 26.8, 1203, '#ffcf99', '#ff8a3d', '炒', 32),
      dish('d102', '肥西老母鸡汤', '现炖 6 小时，鸡肉软烂', 22, 986, '#ffe6a6', '#f0a920', '汤'),
      dish('d103', '香菇滑鸡饭', '鸡腿肉、鲜香菇、秘制酱汁', 21.5, 760, '#ffd8b0', '#e98a45', '鸡')
    ]},
    { name: '招牌套餐', items: [
      dish('d104', '梅菜扣肉套餐', '五花肉蒸足 3 小时', 29.9, 432, '#e8c4a0', '#a5653a', '肉'),
      dish('d105', '鱼香肉丝套餐', '酸甜适口，下饭神器', 24.8, 511, '#ffc2a8', '#e5603b', '鱼'),
      dish('d106', '番茄炒蛋套餐', '新鲜番茄，土鸡蛋', 19.9, 640, '#ffc7b8', '#ef4a3a', '蛋')
    ]},
    { name: '小菜', items: [
      dish('d107', '凉拌黄瓜', '清爽解腻', 6, 380, '#c8f0c0', '#4cb050', '瓜'),
      dish('d108', '卤蛋', '老卤浸泡 12 小时', 3, 925, '#f1d9a8', '#b57a2c', '卤')
    ]},
    { name: '饮品', items: [
      dish('d109', '酸梅汤', '乌梅山楂熬制 500ml', 6, 702, '#e8b0c0', '#9c2f55', '梅'),
      dish('d110', '可口可乐', '330ml 罐装', 4, 450, '#ffb3b3', '#d81e1e', '可')
    ]}
  ],
  s2: [
    { name: '热销', items: [
      dish('d201', '番茄牛腩面', '整块牛腩，番茄浓汤', 36, 820, '#ffc4a8', '#e5532d', '腩'),
      dish('d202', '草本汤麻辣小龙虾拌面', '季节限定，虾仁 8 只', 42, 356, '#ffb0a0', '#d63a2a', '虾')
    ]},
    { name: '汤面', items: [
      dish('d203', '招牌和府捞面', '草本骨汤 + 慢炖小排', 32, 640, '#f5dcb0', '#c48a3a', '面'),
      dish('d204', '松茸鸡汤面', '云南松茸，走地鸡', 38, 288, '#efe0c0', '#a8834a', '菌')
    ]},
    { name: '小食', items: [
      dish('d205', '酱香猪蹄', '软糯脱骨', 18, 410, '#e6c0a0', '#9a5a32', '蹄'),
      dish('d206', '书房小菜拼盘', '四味小菜', 12, 236, '#cdeec6', '#58a84e', '菜')
    ]}
  ],
  s3: [
    { name: '热销', items: [
      dish('d301', '招牌腊汁肉夹馍', '肥瘦相间，馍酥肉香', 14, 1820, '#ffe0a0', '#e09a20', '馍'),
      dish('d302', '肉夹馍 + 擀面皮套餐', '一馍一皮一饮', 26, 960, '#ffd4a0', '#e0782a', '套')
    ]},
    { name: '凉皮面食', items: [
      dish('d303', '秦镇擀面皮', '红油香辣，筋道爽滑', 12, 870, '#ffc8a8', '#e85a30', '皮'),
      dish('d304', '油泼面', '宽面 + 热油泼辣子', 18, 540, '#ffd6a0', '#d8661e', '泼')
    ]},
    { name: '饮品', items: [
      dish('d305', '冰峰橙味汽水', '西安老味道', 5, 1100, '#ffe1a0', '#f39c12', '冰')
    ]}
  ],
  s4: [
    { name: '热销', items: [
      dish('d401', '招牌酸菜鱼（单人份）', '巴沙鱼片 + 老坛酸菜', 39.9, 2100, '#fff1a8', '#e8b417', '鱼', 49),
      dish('d402', '番茄鱼（单人份）', '新疆番茄熬汤底', 39.9, 980, '#ffc0a8', '#e5482d', '茄')
    ]},
    { name: '加料', items: [
      dish('d403', '加鱼片', '150g', 12, 640, '#e0f0ff', '#5aa0e8', '片'),
      dish('d404', '加宽粉', '红薯宽粉', 5, 870, '#efe4d4', '#a8865a', '粉'),
      dish('d405', '米饭', '东北五常大米', 2, 2200, '#f5f5ef', '#b8b8a0', '饭')
    ]},
    { name: '饮品', items: [
      dish('d406', '冰镇酸梅汤', '解辣必备', 6, 760, '#e8b0c0', '#9c2f55', '梅')
    ]}
  ],
  s5: [
    { name: '热销', items: [
      dish('d501', '杨枝甘露', '芒果 + 西柚 + 椰浆', 16, 5200, '#ffe08a', '#f5a300', '芒'),
      dish('d502', '豆乳玉麒麟', '豆乳奶盖 + 黑糖珍珠', 15, 3800, '#f0e0c8', '#a8784a', '豆')
    ]},
    { name: '鲜果茶', items: [
      dish('d503', '西瓜啵啵', '整颗西瓜现榨', 14, 2100, '#ffb8c0', '#e8384c', '瓜'),
      dish('d504', '超级杯水果茶', '7 种鲜果', 18, 1650, '#ffd0a0', '#ef6d2a', '果')
    ]},
    { name: '奶茶', items: [
      dish('d505', '招牌芋圆奶茶', '手作芋圆，Q 弹软糯', 13, 2980, '#e0d0f0', '#8a5ac8', '芋')
    ]}
  ],
  s6: [
    { name: '热销', items: [
      dish('d601', '鲜肉大馄饨', '一碗 12 只，紫菜虾皮汤', 16, 980, '#f5ead8', '#c09060', '馄'),
      dish('d602', '肉串 10 串', '羊肉现穿现烤', 25, 760, '#ffc8a0', '#c8541e', '串')
    ]},
    { name: '米饭套餐', items: [
      dish('d603', '香煎鸡排饭', '鸡排 + 时蔬 + 米饭', 22, 640, '#ffdca8', '#de8a2a', '排'),
      dish('d604', '宫保鸡丁饭', '花生酥脆', 21, 520, '#ffc4a0', '#d65a28', '宫')
    ]}
  ]
};

const categories = [
  { name: '美食', glyph: '食', c1: '#ffb347', c2: '#ff7a1a' },
  { name: '甜品饮品', glyph: '饮', c1: '#ffa6c1', c2: '#e8487c' },
  { name: '超市便利', glyph: '超', c1: '#7cc6ff', c2: '#2a7fff' },
  { name: '蔬菜水果', glyph: '果', c1: '#8fe0a0', c2: '#2eaa5b' },
  { name: '看病买药', glyph: '药', c1: '#7ee0d8', c2: '#13a89e' },
  { name: '浪漫鲜花', glyph: '花', c1: '#ffb0d8', c2: '#d84aa0' },
  { name: '汉堡披萨', glyph: '堡', c1: '#ffd36b', c2: '#f29a00' },
  { name: '跑腿代购', glyph: '跑', c1: '#a8b8ff', c2: '#5a6af0' },
  { name: '夜宵', glyph: '夜', c1: '#b8a0ff', c2: '#7448e0' },
  { name: '全部分类', glyph: '全', c1: '#c8ced8', c2: '#7a8494' }
];

const addresses = [
  { id: 'a1', tag: '公司', name: '陈先生', phone: '138****6620', detail: '望京SOHO T3 座 18 层 1806' },
  { id: 'a2', tag: '家', name: '陈先生', phone: '138****6620', detail: '花家地北里 8 号院 3 号楼 2 单元 502' },
  { id: 'a3', tag: '学校', name: '陈同学', phone: '186****2231', detail: '北京工业大学 南门快递柜旁' }
];

const coupons = [
  { id: 'c0', title: '不使用红包', amount: 0, min: 0 },
  { id: 'c1', title: '无门槛红包 ¥3', amount: 3, min: 0 },
  { id: 'c2', title: '满 30 减 6 通用红包', amount: 6, min: 30 },
  { id: 'c3', title: '满 50 减 12 超级会员红包', amount: 12, min: 50 }
];

function findShop(id) {
  return shops.find(s => s.id === id) || shops[0];
}

function findDish(shopId, dishId) {
  const groups = menus[shopId] || [];
  for (const g of groups) {
    const d = g.items.find(x => x.id === dishId);
    if (d) return d;
  }
  return null;
}

module.exports = { shops, menus, categories, addresses, coupons, findShop, findDish };
