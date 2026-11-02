// 离线演示数据：附近商家、菜单、规格、红包（价格单位：元）
// cats：金刚区分类（首页按它筛）；lat / lng：「商家」页导航用
const shops = [
  {
    id: 's1', name: '老乡鸡（望京SOHO店）', logo: '鸡', c1: '#ffb347', c2: '#ff7a1a',
    cat: '中式快餐', cats: ['美食', '快餐便当'], rating: 4.8, sales: 3021, time: 28, distance: '1.2km',
    minOrder: 20, deliveryFee: 3, packFee: 1, tags: ['满30减8', '满50减15', '新客立减5'], brand: true,
    notice: '坚持现炒现做，肥西老母鸡汤每日现炖 6 小时',
    addr: '朝阳区望京街 10 号望京SOHO B1 层', hours: '09:30 - 22:00', phone: '010-6478 2211', lat: 39.9962, lng: 116.4807
  },
  {
    id: 's2', name: '和府捞面（来福士店）', logo: '面', c1: '#8fd3a4', c2: '#2e9e5b',
    cat: '面馆', cats: ['美食'], rating: 4.7, sales: 1856, time: 32, distance: '1.8km',
    minOrder: 25, deliveryFee: 4, packFee: 2, tags: ['满40减10', '满70减20'], brand: true,
    notice: '书房里的养生面，草本骨汤熬足 12 小时',
    addr: '朝阳区望京来福士 L3-12', hours: '10:00 - 21:30', phone: '010-6471 3366', lat: 39.9938, lng: 116.4741
  },
  {
    id: 's3', name: '西少爷肉夹馍', logo: '馍', c1: '#ffd36b', c2: '#f29a00',
    cat: '小吃', cats: ['美食', '夜宵'], rating: 4.6, sales: 2410, time: 25, distance: '0.9km',
    minOrder: 15, deliveryFee: 2, packFee: 1, tags: ['满25减6', '满45减10'], brand: false,
    notice: '腊汁肉每天凌晨 4 点开炖，馍现烤现夹',
    addr: '朝阳区阜通东大街 6 号方恒购物中心 1 层', hours: '09:00 - 23:30', phone: '010-6439 5521', lat: 39.9921, lng: 116.4778
  },
  {
    id: 's4', name: '鱼你在一起·酸菜鱼', logo: '鱼', c1: '#7cc6ff', c2: '#2a7fff',
    cat: '川湘菜', cats: ['美食', '夜宵'], rating: 4.9, sales: 4120, time: 35, distance: '2.1km',
    minOrder: 30, deliveryFee: 5, packFee: 2, tags: ['满50减18', '满80减28'], brand: true,
    notice: '一人一份酸菜鱼，巴沙鱼片无刺嫩滑',
    addr: '朝阳区广顺北大街 33 号凯德 MALL B1', hours: '10:30 - 次日 01:00', phone: '010-8473 6620', lat: 40.0021, lng: 116.4701
  },
  {
    id: 's5', name: '茶百道（望京街店）', logo: '茶', c1: '#ffa6c1', c2: '#e8487c',
    cat: '奶茶饮品', cats: ['甜品饮品'], rating: 4.8, sales: 9860, time: 22, distance: '0.6km',
    minOrder: 18, deliveryFee: 2, packFee: 1, tags: ['满30减5', '满50减10'], brand: true,
    notice: '鲜果现切，下单后 3 分钟出杯',
    addr: '朝阳区望京街 9 号 1 层 108', hours: '10:00 - 22:30', phone: '010-6470 8899', lat: 39.9955, lng: 116.4822
  },
  {
    id: 's6', name: '南城香（阜通店）', logo: '香', c1: '#ffb59a', c2: '#e2552d',
    cat: '中式快餐', cats: ['美食', '快餐便当', '夜宵'], rating: 4.5, sales: 1530, time: 30, distance: '1.5km',
    minOrder: 20, deliveryFee: 3, packFee: 1, tags: ['满35减9', '满60减15'], brand: false,
    notice: '馄饨、烤串、米饭，北京人的家常味道，24 小时营业',
    addr: '朝阳区阜通西大街 7 号', hours: '24 小时营业', phone: '010-6432 1180', lat: 39.9908, lng: 116.4712
  },
  {
    id: 's7', name: '塔斯汀·中国汉堡（望京店）', logo: '堡', c1: '#ffd36b', c2: '#e8541e',
    cat: '汉堡西餐', cats: ['美食', '汉堡披萨'], rating: 4.7, sales: 2760, time: 26, distance: '1.1km',
    minOrder: 20, deliveryFee: 3, packFee: 1, tags: ['满30减6', '满55减12'], brand: true,
    notice: '手擀现烤堡胚，中国味汉堡',
    addr: '朝阳区望京西路 48 号金隅国际 1 层', hours: '10:00 - 23:00', phone: '010-6476 0718', lat: 39.9975, lng: 116.4693
  },
  {
    id: 's8', name: '便利蜂（望京SOHO店）', logo: '便', c1: '#ffcf6b', c2: '#ff9a1a',
    cat: '便利店', cats: ['超市便利', '夜宵'], rating: 4.7, sales: 5320, time: 20, distance: '0.4km',
    minOrder: 15, deliveryFee: 2, packFee: 0, tags: ['满29减5', '满59减12'], brand: true,
    notice: '24 小时营业，零食饮料日用百货 30 分钟送达',
    addr: '朝阳区望京街 10 号望京SOHO T2 一层', hours: '24 小时营业', phone: '010-6478 5560', lat: 39.9960, lng: 116.4815
  },
  {
    id: 's9', name: '百果园（望京西园店）', logo: '果', c1: '#8fe0a0', c2: '#2eaa5b',
    cat: '水果生鲜', cats: ['蔬菜水果'], rating: 4.8, sales: 3480, time: 30, distance: '1.6km',
    minOrder: 20, deliveryFee: 3, packFee: 0, tags: ['满39减8', '满69减15'], brand: true,
    notice: '不好吃三无退货，果切现切现送',
    addr: '朝阳区望京西园三区 309 号楼底商', hours: '08:00 - 23:00', phone: '010-6473 2290', lat: 39.9989, lng: 116.4662
  },
  {
    id: 's10', name: '叮当快药（24 小时药房）', logo: '药', c1: '#7ee0d8', c2: '#13a89e',
    cat: '药房', cats: ['看病买药', '夜宵'], rating: 4.9, sales: 1960, time: 28, distance: '1.3km',
    minOrder: 0, deliveryFee: 5, packFee: 0, tags: ['满49减6', '夜间不打烊'], brand: true,
    notice: '执业药师 24 小时在线，处方药请凭处方购买',
    addr: '朝阳区望京东路 1 号', hours: '24 小时营业', phone: '400-109-1099', lat: 40.0002, lng: 116.4876
  },
  {
    id: 's11', name: '花点时间（望京花艺店）', logo: '花', c1: '#ffb0d8', c2: '#d84aa0',
    cat: '鲜花', cats: ['浪漫鲜花'], rating: 4.8, sales: 860, time: 45, distance: '2.4km',
    minOrder: 50, deliveryFee: 8, packFee: 0, tags: ['满199减30', '免费贺卡'], brand: true,
    notice: '云南基地直发，花艺师现场包扎',
    addr: '朝阳区望京花家地南街 8 号', hours: '09:00 - 21:00', phone: '010-6477 3355', lat: 39.9853, lng: 116.4730
  }
];

// 规格模板：每组单选，add 为加价（元）。默认选每组第一项
const specs = {
  tea: [
    { name: '杯型', opts: [{ n: '中杯' }, { n: '大杯', add: 3 }] },
    { name: '温度', opts: [{ n: '少冰' }, { n: '正常冰' }, { n: '去冰' }, { n: '热' }] },
    { name: '甜度', opts: [{ n: '七分糖' }, { n: '标准糖' }, { n: '半糖' }, { n: '无糖' }] }
  ],
  spicy: [
    { name: '辣度', opts: [{ n: '微辣' }, { n: '中辣' }, { n: '特辣' }, { n: '不辣' }] }
  ],
  noodle: [
    { name: '面量', opts: [{ n: '标准' }, { n: '加面', add: 3 }] },
    { name: '辣度', opts: [{ n: '不辣' }, { n: '微辣' }, { n: '中辣' }] }
  ],
  fish: [
    { name: '份量', opts: [{ n: '单人份' }, { n: '双人份', add: 30 }] },
    { name: '辣度', opts: [{ n: '微辣' }, { n: '中辣' }, { n: '特辣' }] }
  ],
  rice: [
    { name: '米饭', opts: [{ n: '正常米饭' }, { n: '多加米饭', add: 1 }, { n: '少饭' }] }
  ],
  flower: [
    { name: '贺卡', opts: [{ n: '不需要' }, { n: '附手写贺卡' }] },
    { name: '包装', opts: [{ n: '雾面纸' }, { n: '礼盒装', add: 20 }] }
  ]
};

function dish(id, name, desc, price, sales, c1, c2, glyph, origin, spec) {
  return { id, name, desc, price, sales, c1, c2, glyph, origin: origin || 0, spec: spec || '' };
}

// 每家店的菜单：按分类分组
const menus = {
  s1: [
    { name: '热销', items: [
      dish('d101', '农家小炒肉套餐', '精选前腿肉 + 杭椒，配米饭', 26.8, 1203, '#ffcf99', '#ff8a3d', '炒', 32, 'rice'),
      dish('d102', '肥西老母鸡汤', '现炖 6 小时，鸡肉软烂', 22, 986, '#ffe6a6', '#f0a920', '汤'),
      dish('d103', '香菇滑鸡饭', '鸡腿肉、鲜香菇、秘制酱汁', 21.5, 760, '#ffd8b0', '#e98a45', '鸡', 0, 'rice')
    ]},
    { name: '招牌套餐', items: [
      dish('d104', '梅菜扣肉套餐', '五花肉蒸足 3 小时', 29.9, 432, '#e8c4a0', '#a5653a', '肉', 0, 'rice'),
      dish('d105', '鱼香肉丝套餐', '酸甜适口，下饭神器', 24.8, 511, '#ffc2a8', '#e5603b', '鱼', 0, 'rice'),
      dish('d106', '番茄炒蛋套餐', '新鲜番茄，土鸡蛋', 19.9, 640, '#ffc7b8', '#ef4a3a', '蛋', 0, 'rice')
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
      dish('d201', '番茄牛腩面', '整块牛腩，番茄浓汤', 36, 820, '#ffc4a8', '#e5532d', '腩', 0, 'noodle'),
      dish('d202', '草本汤麻辣小龙虾拌面', '季节限定，虾仁 8 只', 42, 356, '#ffb0a0', '#d63a2a', '虾', 0, 'noodle')
    ]},
    { name: '汤面', items: [
      dish('d203', '招牌和府捞面', '草本骨汤 + 慢炖小排', 32, 640, '#f5dcb0', '#c48a3a', '面', 0, 'noodle'),
      dish('d204', '松茸鸡汤面', '云南松茸，走地鸡', 38, 288, '#efe0c0', '#a8834a', '菌', 0, 'noodle')
    ]},
    { name: '小食', items: [
      dish('d205', '酱香猪蹄', '软糯脱骨', 18, 410, '#e6c0a0', '#9a5a32', '蹄'),
      dish('d206', '书房小菜拼盘', '四味小菜', 12, 236, '#cdeec6', '#58a84e', '菜')
    ]}
  ],
  s3: [
    { name: '热销', items: [
      dish('d301', '招牌腊汁肉夹馍', '肥瘦相间，馍酥肉香', 14, 1820, '#ffe0a0', '#e09a20', '馍'),
      dish('d302', '肉夹馍 + 擀面皮套餐', '一馍一皮一饮', 26, 960, '#ffd4a0', '#e0782a', '套', 0, 'spicy')
    ]},
    { name: '凉皮面食', items: [
      dish('d303', '秦镇擀面皮', '红油香辣，筋道爽滑', 12, 870, '#ffc8a8', '#e85a30', '皮', 0, 'spicy'),
      dish('d304', '油泼面', '宽面 + 热油泼辣子', 18, 540, '#ffd6a0', '#d8661e', '泼', 0, 'spicy')
    ]},
    { name: '饮品', items: [
      dish('d305', '冰峰橙味汽水', '西安老味道', 5, 1100, '#ffe1a0', '#f39c12', '冰')
    ]}
  ],
  s4: [
    { name: '热销', items: [
      dish('d401', '招牌酸菜鱼', '巴沙鱼片 + 老坛酸菜', 39.9, 2100, '#fff1a8', '#e8b417', '鱼', 49, 'fish'),
      dish('d402', '番茄鱼', '新疆番茄熬汤底', 39.9, 980, '#ffc0a8', '#e5482d', '茄', 0, 'fish')
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
      dish('d501', '杨枝甘露', '芒果 + 西柚 + 椰浆', 16, 5200, '#ffe08a', '#f5a300', '芒', 0, 'tea'),
      dish('d502', '豆乳玉麒麟', '豆乳奶盖 + 黑糖珍珠', 15, 3800, '#f0e0c8', '#a8784a', '豆', 0, 'tea')
    ]},
    { name: '鲜果茶', items: [
      dish('d503', '西瓜啵啵', '整颗西瓜现榨', 14, 2100, '#ffb8c0', '#e8384c', '瓜', 0, 'tea'),
      dish('d504', '超级杯水果茶', '7 种鲜果', 18, 1650, '#ffd0a0', '#ef6d2a', '果', 0, 'tea')
    ]},
    { name: '奶茶', items: [
      dish('d505', '招牌芋圆奶茶', '手作芋圆，Q 弹软糯', 13, 2980, '#e0d0f0', '#8a5ac8', '芋', 0, 'tea')
    ]}
  ],
  s6: [
    { name: '热销', items: [
      dish('d601', '鲜肉大馄饨', '一碗 12 只，紫菜虾皮汤', 16, 980, '#f5ead8', '#c09060', '馄'),
      dish('d602', '肉串 10 串', '羊肉现穿现烤', 25, 760, '#ffc8a0', '#c8541e', '串', 0, 'spicy')
    ]},
    { name: '米饭套餐', items: [
      dish('d603', '香煎鸡排饭', '鸡排 + 时蔬 + 米饭', 22, 640, '#ffdca8', '#de8a2a', '排', 0, 'rice'),
      dish('d604', '宫保鸡丁饭', '花生酥脆', 21, 520, '#ffc4a0', '#d65a28', '宫', 0, 'rice')
    ]}
  ],
  s7: [
    { name: '热销', items: [
      dish('d701', '香辣鸡腿中国汉堡', '手擀堡胚 + 整块鸡腿肉', 16.9, 1680, '#ffd9a0', '#e8741e', '堡', 21.9, 'spicy'),
      dish('d702', '藤椒双鸡中国汉堡', '双层鸡排，藤椒麻香', 19.9, 920, '#e8f0b0', '#8ca83a', '藤', 0, 'spicy')
    ]},
    { name: '套餐', items: [
      dish('d703', '单人超值套餐', '汉堡 + 薯条 + 可乐', 29.9, 1150, '#ffc8a0', '#e0562a', '套')
    ]},
    { name: '小食饮品', items: [
      dish('d704', '黄金薯条', '现炸，外酥里糯', 8, 1320, '#ffe7a0', '#e8a81e', '薯'),
      dish('d705', '香酥鸡块 5 块', '蘸酱二选一', 9.9, 860, '#ffdcb0', '#d9822e', '块'),
      dish('d706', '可乐（中）', '冰爽 500ml', 6, 990, '#ffb3b3', '#d81e1e', '可')
    ]}
  ],
  s8: [
    { name: '热销', items: [
      dish('d801', '农夫山泉 550ml', '天然饮用水', 2, 3200, '#c8e6ff', '#3a8ee8', '水'),
      dish('d802', '康师傅红烧牛肉面', '经典桶装', 5.5, 1860, '#ffc8a0', '#e0562a', '面')
    ]},
    { name: '零食', items: [
      dish('d803', '乐事原味薯片 75g', '休闲零食', 7.9, 1420, '#fff0a0', '#e8c41e', '薯'),
      dish('d804', '奥利奥夹心饼干', '原味 97g', 9.9, 960, '#d0d4e0', '#3a4a6a', '奥')
    ]},
    { name: '乳饮日用', items: [
      dish('d805', '蒙牛纯牛奶 250ml×4', '早餐好伴侣', 13.9, 740, '#f5f5ff', '#8a9ad8', '奶'),
      dish('d806', '维达抽纸 3 包', '三层 100 抽', 12.9, 650, '#e8f4ff', '#5a9ad8', '纸')
    ]}
  ],
  s9: [
    { name: '当季鲜果', items: [
      dish('d901', '阳光玫瑰葡萄 500g', '云南产区，脆甜无籽', 29.9, 1280, '#d8f0a8', '#6ab02e', '葡', 39.9),
      dish('d902', '丹东草莓 400g', '红颜品种，香甜多汁', 39.9, 860, '#ffb8c0', '#e8384c', '莓')
    ]},
    { name: '果切', items: [
      dish('d903', '西瓜果切 500g', '现切现送，冰镇口感', 15.9, 2150, '#ffc0c8', '#e84a5a', '瓜'),
      dish('d904', '什锦果切 400g', '五种鲜果拼盘', 19.9, 1430, '#ffe0a0', '#f0a020', '拼')
    ]},
    { name: '新鲜蔬菜', items: [
      dish('d905', '西兰花 500g', '当日到货', 6.9, 520, '#c8f0c0', '#3ea35a', '兰'),
      dish('d906', '普罗旺斯番茄 500g', '沙瓤多汁', 8.9, 610, '#ffc7b8', '#ef4a3a', '茄')
    ]}
  ],
  s10: [
    { name: '感冒发烧', items: [
      dish('d1001', '布洛芬缓释胶囊 24 粒', '退热、缓解轻中度疼痛', 16.8, 1320, '#c8f0ec', '#13a89e', '布'),
      dish('d1002', '连花清瘟胶囊 36 粒', '清瘟解毒，宣肺泄热', 14.5, 980, '#d8f0c8', '#4caa50', '连')
    ]},
    { name: '日常护理', items: [
      dish('d1003', '医用外科口罩 10 只', '三层防护，独立包装', 9.9, 2460, '#d0e8ff', '#3a8ee8', '罩'),
      dish('d1004', '创可贴 20 片', '防水透气', 8, 760, '#ffe0c8', '#e8843a', '贴'),
      dish('d1005', '电子体温计', '15 秒快速测温', 29, 420, '#e0e0f8', '#6a6ad8', '温')
    ]}
  ],
  s11: [
    { name: '热销花束', items: [
      dish('d1101', '11 枝红玫瑰花束', '卡罗拉红玫瑰，一心一意', 129, 420, '#ffb0c0', '#d8284a', '玫', 169, 'flower'),
      dish('d1102', '向日葵花束', '3 枝向日葵 + 配花', 99, 310, '#ffe08a', '#f0a000', '葵', 0, 'flower'),
      dish('d1103', '康乃馨花束', '粉色康乃馨 12 枝', 89, 260, '#ffc8e0', '#e05a9a', '康', 0, 'flower')
    ]},
    { name: '绿植', items: [
      dish('d1104', '多肉组合盆栽', '四款多肉 + 陶盆', 39, 520, '#c8f0c0', '#3ea35a', '肉')
    ]}
  ]
};

// 金刚区：cat 为空 = 显示全部
const categories = [
  { name: '美食', glyph: '食', c1: '#ffb347', c2: '#ff7a1a' },
  { name: '甜品饮品', glyph: '饮', c1: '#ffa6c1', c2: '#e8487c' },
  { name: '超市便利', glyph: '超', c1: '#7cc6ff', c2: '#2a7fff' },
  { name: '蔬菜水果', glyph: '果', c1: '#8fe0a0', c2: '#2eaa5b' },
  { name: '看病买药', glyph: '药', c1: '#7ee0d8', c2: '#13a89e' },
  { name: '浪漫鲜花', glyph: '花', c1: '#ffb0d8', c2: '#d84aa0' },
  { name: '汉堡披萨', glyph: '堡', c1: '#ffd36b', c2: '#f29a00' },
  { name: '快餐便当', glyph: '餐', c1: '#a8b8ff', c2: '#5a6af0' },
  { name: '夜宵', glyph: '夜', c1: '#b8a0ff', c2: '#7448e0' },
  { name: '全部分类', glyph: '全', c1: '#c8ced8', c2: '#7a8494' }
];

// 首次打开的收货地址（之后以 storage 为准，可增删改）
const addresses = [
  { id: 'a1', tag: '公司', name: '陈小饱', gender: '先生', phone: '13812346620', region: ['北京市', '北京市', '朝阳区'], detail: '望京SOHO T3 座', house: '18 层 1806', isDefault: true },
  { id: 'a2', tag: '家', name: '陈小饱', gender: '先生', phone: '13812346620', region: ['北京市', '北京市', '朝阳区'], detail: '花家地北里 8 号院', house: '3 号楼 2 单元 502', isDefault: false },
  { id: 'a3', tag: '学校', name: '陈同学', gender: '女士', phone: '18601232231', region: ['北京市', '北京市', '朝阳区'], detail: '北京工业大学', house: '南门快递柜旁', isDefault: false }
];

// 红包（平台通用）与商家代金券。有效期按「今天 + days」算，演示数据不会过期
const coupons = [
  { id: 'r1', kind: 'red', title: '无门槛红包', amount: 3, min: 0, days: 6 },
  { id: 'r2', kind: 'red', title: '午餐红包', amount: 4, min: 20, days: 3 },
  { id: 'r3', kind: 'red', title: '通用红包', amount: 6, min: 30, days: 10 },
  { id: 'r4', kind: 'red', title: '大额红包', amount: 12, min: 50, days: 15 },
  { id: 'r0', kind: 'red', title: '周末红包', amount: 5, min: 25, days: -2 },
  { id: 'k1', kind: 'shop', shopId: 's1', title: '老乡鸡代金券', amount: 5, min: 40, days: 20 },
  { id: 'k2', kind: 'shop', shopId: 's4', title: '鱼你在一起代金券', amount: 10, min: 60, days: 12 },
  { id: 'k3', kind: 'shop', shopId: 's5', title: '茶百道代金券', amount: 3, min: 25, days: 8 }
];

// 吃货豆兑红包
const beanGoods = [
  { id: 'b1', beans: 300, amount: 2, min: 0, title: '无门槛红包' },
  { id: 'b2', beans: 800, amount: 6, min: 30, title: '满 30 可用红包' },
  { id: 'b3', beans: 1500, amount: 12, min: 50, title: '满 50 可用大额红包' }
];

const recharges = [
  { pay: 50, bonus: 0 },
  { pay: 100, bonus: 5 },
  { pay: 200, bonus: 15 }
];

const rateTags = ['味道赞', '分量足', '包装好', '送餐快', '骑手热情', '性价比高'];

const refundReasons = ['不想要了', '商家出餐太慢', '地址 / 电话填错了', '点错了菜品', '其他原因'];

const faqs = [
  { q: '订单可以取消吗？', a: '商家接单前可随时取消，款项立即原路退回；商家接单后可在订单详情「联系客服」里申请退款，红包与吃货豆一并退回。' },
  { q: '红包怎么用？', a: '提交订单时自动选择最划算的红包，也可以点「红包」行手动挑选。商家代金券仅限对应店铺使用，与红包可叠加。' },
  { q: '超级会员有什么权益？', a: '开通后立即得 4 张 ¥5 无门槛红包，有效期至会员到期日；会员期内可随时续费，红包按月发放。' },
  { q: '余额可以提现吗？', a: '充值金额可在下单时直接抵扣，订单退款原路退回余额；赠送金额不可提现。' },
  { q: '吃货豆怎么得？', a: '每消费 1 元得 1 个吃货豆，评价订单再得 20 个；吃货豆可在「吃货豆」页兑换红包。' },
  { q: '送达超时怎么办？', a: '饱了么专送订单享准时宝保障，超时 15 分钟以上可联系客服申请赔付红包。' }
];

const SERVICE_PHONE = '400-820-8820';
const RIDER = { name: '张师傅', phone: '17012348812' };

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

module.exports = {
  shops, specs, menus, categories, addresses, coupons, beanGoods, recharges, rateTags, refundReasons, faqs,
  SERVICE_PHONE, RIDER, findShop, findDish
};
