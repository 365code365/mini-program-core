// 门店、项目、发型师、作品与评价的静态数据（真实门店会从后台拉，这里固化成一份可信的样例）
const store = {
  name: '型格美业·万象城店',
  rating: 4.9,
  reviews: 3862,
  hours: '10:00 - 21:30',
  address: '杭州市上城区富春路701号万象城B1层B127',
  phone: '0571-86912368',
  latitude: 30.2468,
  longitude: 120.2159,
  tags: ['免费停车3小时', '提供饮品', '可刷储值卡', '支持预约']
};

const categories = [
  { id: 'all', name: '全部' },
  { id: 'cut', name: '剪发' },
  { id: 'dye', name: '染发' },
  { id: 'perm', name: '烫发' },
  { id: 'care', name: '护理' },
  { id: 'nail', name: '美甲' },
  { id: 'lash', name: '美睫' }
];

const services = [
  { id: 'c1', cat: 'cut', name: '设计师洗剪吹', price: 88, duration: 60, sold: 5213, desc: '洗发 + 头部按摩 + 设计剪裁 + 造型吹风', hot: true },
  { id: 'c2', cat: 'cut', name: '总监洗剪吹', price: 168, duration: 60, sold: 2307, desc: '创意总监一对一沟通脸型，定制发型方案', hot: true },
  { id: 'c3', cat: 'cut', name: '男士精剪', price: 68, duration: 45, sold: 3920, desc: '渐变铲青 / 纹理碎发，含洗发与发蜡造型' },
  { id: 'c4', cat: 'cut', name: '儿童理发（12岁以下）', price: 48, duration: 30, sold: 1186, desc: '安抚式理发，配儿童围布与动画屏' },
  { id: 'd1', cat: 'dye', name: '时尚染发（短发）', price: 398, duration: 120, sold: 1654, desc: '欧莱雅专业染膏，奶茶棕 / 冷棕 / 亚麻色可选', hot: true },
  { id: 'd2', cat: 'dye', name: '雾霾蓝漂染', price: 688, duration: 180, sold: 742, desc: '两次漂发 + 护色锁色，含漂后修复护理' },
  { id: 'd3', cat: 'dye', name: '植物染黑', price: 298, duration: 90, sold: 963, desc: '指甲花植物染膏，遮盖白发，孕期可做' },
  { id: 'p1', cat: 'perm', name: '韩式空气烫', price: 588, duration: 150, sold: 1388, desc: '蓬松空气感卷度，含烫前护理与定型', hot: true },
  { id: 'p2', cat: 'perm', name: '数码烫', price: 688, duration: 180, sold: 816, desc: '大波浪 / 羊毛卷，弹力持久约 6 个月' },
  { id: 'p3', cat: 'perm', name: '男士纹理烫', price: 398, duration: 120, sold: 1027, desc: '韩式纹理 / 锡纸烫，蓬松有型好打理' },
  { id: 'h1', cat: 'care', name: '头皮深层清洁', price: 168, duration: 50, sold: 2140, desc: '头皮检测 + 去角质 + 舒缓精华按摩' },
  { id: 'h2', cat: 'care', name: '日本酸性护理', price: 268, duration: 60, sold: 1302, desc: '修复烫染受损，改善毛躁分叉，柔顺有光泽' },
  { id: 'n1', cat: 'nail', name: '纯色美甲', price: 128, duration: 60, sold: 2980, desc: '日式光疗胶，含修型与死皮处理，可保持 3~4 周', hot: true },
  { id: 'n2', cat: 'nail', name: '猫眼款式美甲', price: 198, duration: 90, sold: 1430, desc: '磁吸猫眼 + 跳色设计，送手部护理' },
  { id: 'n3', cat: 'nail', name: '法式美甲', price: 168, duration: 75, sold: 1105, desc: '经典法式白边 / 渐变法式，通勤百搭' },
  { id: 'l1', cat: 'lash', name: '自然款嫁接睫毛', price: 198, duration: 90, sold: 1876, desc: '0.07 山茶花睫毛，单根嫁接约 120 根' },
  { id: 'l2', cat: 'lash', name: '下睫毛嫁接', price: 98, duration: 40, sold: 655, desc: '根根分明的漫画下睫毛，与上睫毛搭配更自然' }
];

const steps = {
  cut: ['发质与脸型沟通', '洗发 + 头部按摩 10 分钟', '设计剪裁', '吹风造型 + 打理指导'],
  dye: ['发色方案沟通与试色', '头皮隔离防护', '涂抹染膏并计时', '冲洗 + 锁色护理', '吹风造型'],
  perm: ['卷度方案沟通', '烫前修复护理', '上杠 / 软化 / 定型', '冲洗 + 护理', '吹风造型'],
  care: ['头皮 / 发质检测', '深层清洁', '精华导入与按摩', '冲洗吹干'],
  nail: ['手部清洁与消毒', '修型 + 死皮处理', '上底胶 / 色胶 / 封层', '照灯固化 + 手部护理'],
  lash: ['眼型沟通', '隔离下睫毛', '逐根嫁接', '整理定型 + 护理说明']
};

const notices = [
  '请提前 10 分钟到店，迟到超过 15 分钟门店可能顺延或改约',
  '预约可在开始前 2 小时免费取消或改约',
  '染烫项目价格按短发计，中长发到店由设计师评估后加收 ¥100~200',
  '孕期、头皮破损或过敏体质请提前告知设计师'
];

const stylists = [
  { id: 's1', name: 'Kevin 陈', title: '创意总监', years: 12, rating: 4.9, orders: 6820, avatar: '/images/avatar/s1.jpg',
    cats: ['cut', 'perm', 'dye', 'care'], tags: ['男士纹理', '渐变铲青', '脸型修饰'], works: ['w1', 'w5', 'w3'],
    intro: '前沙宣学院讲师，伦敦进修归来。擅长根据头骨与脸型设计好打理的短发。' },
  { id: 's2', name: 'Lily 林', title: '首席发型师', years: 8, rating: 4.8, orders: 4310, avatar: '/images/avatar/s2.jpg',
    cats: ['cut', 'perm', 'dye', 'care'], tags: ['日系卷发', '法式刘海', '空气感'], works: ['w3', 'w6', 'w2'],
    intro: '东京 Peek-a-boo 研修，擅长温柔系卷发和低维护的层次剪裁。' },
  { id: 's3', name: '阿杰', title: '高级发型师', years: 6, rating: 4.7, orders: 3022, avatar: '/images/avatar/s3.jpg',
    cats: ['cut', 'perm', 'care'], tags: ['复古油头', '男士锡纸烫', '学生头'], works: ['w5', 'w1'],
    intro: '男士发型专精，剪得快也剪得细，周末档期最抢手。' },
  { id: 's4', name: 'Mia 苏', title: '色彩总监', years: 10, rating: 4.9, orders: 3976, avatar: '/images/avatar/s4.jpg',
    cats: ['dye', 'care', 'cut'], tags: ['漂染', '挑染', '雾霾色系'], works: ['w2', 'w4', 'w6'],
    intro: '欧莱雅色彩认证讲师，擅长低饱和冷色调与白发遮盖方案。' },
  { id: 's5', name: 'Coco 张', title: '美甲美睫师', years: 5, rating: 4.8, orders: 2654, avatar: '/images/avatar/s5.jpg',
    cats: ['nail', 'lash'], tags: ['猫眼', '手绘', '自然款睫毛'], works: ['w7', 'w8'],
    intro: '日本 JNA 美甲二级认证，手稳细致，睫毛嫁接不刺激不闷眼。' }
];

const works = [
  { id: 'w1', src: '/images/works/w1.jpg', title: '男士纹理短发', by: 's1' },
  { id: 'w2', src: '/images/works/w2.jpg', title: '雾霾蓝波波头', by: 's4' },
  { id: 'w3', src: '/images/works/w3.jpg', title: '奶茶棕空气卷', by: 's2' },
  { id: 'w4', src: '/images/works/w4.jpg', title: '黑茶色长直发', by: 's4' },
  { id: 'w5', src: '/images/works/w5.jpg', title: '清爽寸头', by: 's3' },
  { id: 'w6', src: '/images/works/w6.jpg', title: '酒红色长发', by: 's2' },
  { id: 'w7', src: '/images/works/w7.jpg', title: '莓果红猫眼甲', by: 's5' },
  { id: 'w8', src: '/images/works/w8.jpg', title: '克莱因蓝纯色甲', by: 's5' }
];

const reviews = [
  { id: 'r1', stylist: 's1', user: '程**', score: 5, date: '2026-10-28', service: '总监洗剪吹', text: 'Kevin 很会沟通，按我的头型剪的短发，回家自己吹也很好打理。' },
  { id: 'r2', stylist: 's1', user: '周**', score: 4.5, date: '2026-10-19', service: '男士纹理烫', text: '卷度自然不油腻，就是周末人多等了 10 分钟。' },
  { id: 'r3', stylist: 's2', user: '许**', score: 5, date: '2026-10-26', service: '韩式空气烫', text: '想要的蓬松感完全做出来了，护理也做得很仔细。' },
  { id: 'r4', stylist: 's2', user: 'Amy', score: 4.5, date: '2026-10-11', service: '设计师洗剪吹', text: '法式刘海剪得刚刚好，不会扎眼睛。' },
  { id: 'r5', stylist: 's3', user: '王**', score: 5, date: '2026-10-30', service: '男士精剪', text: '剪得又快又好，铲青过渡很干净。' },
  { id: 'r6', stylist: 's4', user: '林**', score: 5, date: '2026-10-24', service: '雾霾蓝漂染', text: '漂了两次头发也没有很干，颜色比图片还好看！' },
  { id: 'r7', stylist: 's4', user: '赵**', score: 4, date: '2026-10-08', service: '植物染黑', text: '白发盖得很好，味道也不刺鼻，就是时间稍长。' },
  { id: 'r8', stylist: 's5', user: '小鹿', score: 5, date: '2026-10-29', service: '猫眼款式美甲', text: '猫眼的光泽超级好看，三周了还没翘边。' },
  { id: 'r9', stylist: 's5', user: '陈**', score: 4.5, date: '2026-10-15', service: '自然款嫁接睫毛', text: '很自然，不扎眼，同事都没看出来是嫁接的。' }
];

// 会员等级按累计消费（成长值）划分
const levels = [
  { name: '银卡会员', min: 0, discount: 0.95, perks: ['项目 95 折', '生日赠头皮清洁一次'] },
  { name: '金卡会员', min: 2000, discount: 0.9, perks: ['项目 9 折', '每月 1 次免费吹造型', '生日赠护理一次'] },
  { name: '铂金会员', min: 5000, discount: 0.88, perks: ['项目 88 折', '优先预约总监', '每月 2 次免费吹造型'] },
  { name: '黑钻会员', min: 10000, discount: 0.85, perks: ['项目 85 折', '专属发型顾问', '节假日不限档期'] }
];

const packages = [
  { id: 'k1', pay: 500, gift: 50 },
  { id: 'k2', pay: 1000, gift: 150 },
  { id: 'k3', pay: 2000, gift: 400 },
  { id: 'k4', pay: 5000, gift: 1200 }
];

const catCover = { cut: '/images/svc/cut.jpg', dye: '/images/svc/dye.jpg', perm: '/images/svc/perm.jpg',
  care: '/images/svc/care.jpg', nail: '/images/svc/nail.jpg', lash: '/images/svc/lash.jpg' };

function service(id) { return services.find(s => s.id === id) || null; }
function stylist(id) { return stylists.find(s => s.id === id) || null; }
function work(id) { return works.find(w => w.id === id) || null; }
function catName(id) { const c = categories.find(x => x.id === id); return c ? c.name : ''; }

module.exports = { store, categories, services, steps, notices, stylists, works, reviews, levels, packages, catCover,
  service, stylist, work, catName };
