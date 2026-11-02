// 离线数据：城市、酒店、房型、点评、优惠券。房型的 max 是每间可住人数；hours 有值的是钟点房（按次计价）
const CITIES = [
  { name: '杭州', code: '0571', lat: 30.2741, lng: 120.1551 },
  { name: '上海', code: '021', lat: 31.2304, lng: 121.4737 },
  { name: '北京', code: '010', lat: 39.9042, lng: 116.4074 },
  { name: '成都', code: '028', lat: 30.5728, lng: 104.0668 },
  { name: '三亚', code: '0898', lat: 18.2528, lng: 109.512 },
  { name: '厦门', code: '0592', lat: 24.4798, lng: 118.0894 },
  { name: '大理', code: '0872', lat: 25.6065, lng: 100.2676 },
  { name: '西安', code: '029', lat: 34.3416, lng: 108.9398 }
];
const STAR_TEXT = { 5: '豪华型', 4: '高档型', 3: '舒适型', 2: '经济型', 0: '特色民宿' };
const RATE_TAGS = ['干净卫生', '服务热情', '位置方便', '早餐丰富', '安静舒适', '性价比高'];
// t 显示文字，s 时段开始，e 时段截止（当天入住时截止已过的时段不能再选；最后一档总能选）
const ARRIVALS = [
  { t: '14:00 前', s: '12:00', e: '14:00' }, { t: '14:00-16:00', s: '14:00', e: '16:00' }, { t: '16:00-18:00', s: '16:00', e: '18:00' },
  { t: '18:00-20:00', s: '18:00', e: '20:00' }, { t: '20:00-22:00', s: '20:00', e: '22:00' }, { t: '22:00-24:00', s: '22:00', e: '23:59' },
  { t: '次日 06:00 前', s: '23:59', e: '' }
];
// 某天入住时最早还能选的到店时段下标
function firstArrival(checkin, ts, now) {
  return ARRIVALS.findIndex(a => !a.e || ts(checkin, a.e) > now);
}
const HOUR_STARTS = ['10:00', '12:00', '14:00', '16:00', '18:00', '20:00'];
const NEEDS = ['高楼层', '无烟房', '安静房', '加床（另付费）', '婴儿床'];
const SERVICE_PHONE = '400-820-6688';
const faqs = [
  { q: '订单可以免费取消吗？', a: '入住前一天 18:00 前（当天的订单在下单 30 分钟内）可免费取消，款项、优惠券与积分原路退回；超过时间取消将扣除首晚房费。' },
  { q: '可以开发票吗？', a: '下单时打开「需要发票」并填写抬头，离店后由酒店开具电子发票，发到预订手机号。' },
  { q: '到店找不到订单怎么办？', a: '请向前台报预订人姓名与手机号；仍有问题可在订单详情里「联系酒店」，或拨打途栖客服。' },
  { q: '积分怎么用？', a: '签到、住店都能得积分，预订时 100 积分抵 1 元，单笔最多抵 50 元且不超过房费的 10%。' }
];
// 优惠券模板：days 是从今天起的有效天数（负数 = 已过期）；type 有值时只能用于该类型
const COUPONS = [
  { id: 'k1', amount: 50, min: 300, title: '新客立减 50 元', cond: '满 300 元可用', days: 30 },
  { id: 'k2', amount: 80, min: 800, title: '住宿满 800 减 80', cond: '满 800 元可用', days: 45 },
  { id: 'k3', amount: 30, min: 500, type: '民宿', title: '民宿专享 30 元', cond: '限特色民宿 · 满 500 元', days: 20 },
  { id: 'k4', amount: 20, min: 100, hourly: true, title: '钟点房立减 20 元', cond: '限钟点房 · 满 100 元', days: 15 },
  { id: 'k0', amount: 100, min: 1000, title: '国庆特惠 100 元', cond: '满 1000 元可用', days: -26 }
];

// 房型模板：按星级给
function roomsOf(star, price, img) {
  const r = (id, name, bed, size, breakfast, p, left, max) => ({ id, name, bed, size, breakfast, price: p, left, img: '/images/r' + ((img + Number(id.slice(1))) % 3 + 1) + '.jpg', max });
  if (star === 5) return [r('r1', '豪华大床房', '2.0m大床', '45㎡', '双早', price, 4, 2), r('r2', '豪华双床房', '1.35m双床', '45㎡', '双早', price + 100, 3, 2), r('r3', '行政套房', '2.0m大床', '88㎡', '双早', price * 2, 1, 3)];
  if (star === 4) return [r('r1', '高级大床房', '1.8m大床', '32㎡', '双早', price, 5, 2), r('r2', '高级双床房', '1.2m双床', '32㎡', '双早', price + 60, 3, 2)];
  if (star === 3) return [r('r1', '高级大床房', '1.8m大床', '25㎡', '无早', price, 8, 2), r('r2', '高级双床房', '1.2m双床', '26㎡', '单早', price + 40, 5, 2)];
  if (star === 2) return [r('r1', '标准大床房', '1.5m大床', '18㎡', '无早', price, 10, 2), r('r2', '标准双床房', '1.2m双床', '20㎡', '无早', price + 20, 7, 2)];
  return [r('r1', '庭院大床房', '1.8m大床', '30㎡', '单早', price, 2, 2), r('r2', '亲子 loft', '1.8m+1.2m', '56㎡', '双早', price + 400, 1, 4)];
}
const FAC = {
  5: ['免费WiFi', '室内泳池', '健身房', '中餐厅', '行政酒廊', 'SPA', '接机服务', '24h前台'],
  4: ['免费WiFi', '健身房', '早餐', '洗衣房', '行李寄存', '24h前台'],
  3: ['免费WiFi', '自助洗衣', '行李寄存', '早餐', '24h前台'],
  2: ['免费WiFi', '行李寄存', '24h前台'],
  0: ['免费WiFi', '私家庭院', '茶室', '厨房', '投影', '免费停车']
};
// [id, 城市, 名称, 星级, 商圈, 地址, 距离, 起价, 图, 标签, 评分, 点评数, 有钟点房]
const RAW = [
  ['sh1', '上海', '上海外滩茂悦大酒店', 5, '外滩', '黄浦区黄浦路199号', '距外滩 0.3km', 1580, 4, ['江景', '行政酒廊', '近地铁'], 4.8, 5120, false],
  ['sh2', '上海', '全季酒店（上海人民广场店）', 3, '人民广场', '黄浦区西藏中路318号', '距人民广场站 300m', 469, 3, ['近地铁', '自助洗衣'], 4.7, 6302, true],
  ['sh3', '上海', '隐居·武康路洋房民宿', 0, '武康路', '徐汇区武康路100弄', '距武康大楼 200m', 980, 2, ['老洋房', '可做饭'], 4.9, 862, false],
  ['bj1', '北京', '北京王府井文华东方酒店', 5, '王府井', '东城区王府井大街269号', '距故宫 1.2km', 2280, 1, ['近故宫', 'SPA', '含早'], 4.9, 3311, false],
  ['bj2', '北京', '亚朵酒店（北京三里屯店）', 4, '三里屯', '朝阳区工体北路8号', '距三里屯太古里 300m', 799, 5, ['近商圈', '24h阅读'], 4.8, 4126, true],
  ['bj3', '北京', '汉庭酒店（北京西站店）', 2, '北京西站', '丰台区莲花池东路118号', '距北京西站 500m', 299, 6, ['近火车站', '性价比'], 4.5, 8920, true],
  ['cd1', '成都', '成都太古里博舍酒店', 5, '春熙路', '锦江区笔帖式街81号', '距太古里 100m', 1680, 4, ['设计酒店', '近太古里'], 4.9, 2744, false],
  ['cd2', '成都', '宽窄巷子·院落民宿', 0, '宽窄巷子', '青羊区长顺上街126号', '距宽窄巷子 300m', 520, 2, ['川西院落', '可泡茶'], 4.8, 1290, false],
  ['sy1', '三亚', '三亚亚龙湾瑞吉度假酒店', 5, '亚龙湾', '吉阳区亚龙湾国家旅游度假区', '距海滩 50m', 2680, 1, ['海景房', '私人沙滩', '儿童乐园'], 4.9, 3952, false],
  ['sy2', '三亚', '三亚湾椰林海景公寓', 3, '三亚湾', '天涯区三亚湾路108号', '距海边 200m', 459, 3, ['海景', '可做饭'], 4.6, 2210, true],
  ['xm1', '厦门', '鼓浪屿·海上花园别墅', 0, '鼓浪屿', '思明区鼓浪屿福建路32号', '距三丘田码头 400m', 860, 2, ['老别墅', '海景露台'], 4.8, 1533, false],
  ['xm2', '厦门', '全季酒店（厦门中山路店）', 3, '中山路', '思明区中山路200号', '距中山路步行街 100m', 359, 5, ['近步行街', '近轮渡'], 4.7, 3876, true],
  ['dl1', '大理', '洱海天域·海景民宿', 0, '双廊', '大理市双廊镇环海路18号', '距洱海 20m', 780, 2, ['洱海景', '露台', '可做饭'], 4.9, 1408, false],
  ['dl2', '大理', '大理古城·花间堂', 4, '大理古城', '大理市复兴路233号', '距古城南门 300m', 689, 5, ['古城内', '庭院'], 4.8, 2016, false],
  ['xa1', '西安', '西安钟楼索菲特人民大厦', 5, '钟楼', '新城区东新街319号', '距钟楼 600m', 1080, 4, ['近钟楼', '含早', '室内泳池'], 4.8, 4581, false],
  ['xa2', '西安', '汉庭酒店（西安回民街店）', 2, '钟鼓楼', '莲湖区北院门街66号', '距回民街 100m', 239, 6, ['近回民街', '性价比'], 4.5, 7340, true]
];
const HZ = [
  { id: 'h1', name: '杭州西湖国宾馆·湖景楼', star: 5, score: 4.9, reviews: 3826, area: '西湖风景区', addr: '西湖区杨公堤18号', dist: '距西湖 0.3km', price: 1680, img: '/images/h1.jpg', images: ['/images/h1.jpg', '/images/r1.jpg', '/images/r2.jpg'], tags: ['湖景房', '免费停车', '含早'],
    facilities: ['免费WiFi', '室外泳池', '健身房', '中餐厅', '行李寄存', '接机服务', '会议厅', '24h前台'],
    rooms: [{ id: 'r1', name: '湖景豪华大床房', bed: '1.8m大床', size: '45㎡', breakfast: '双早', price: 1680, left: 3, img: '/images/r1.jpg' }, { id: 'r2', name: '园景高级双床房', bed: '1.2m双床', size: '40㎡', breakfast: '双早', price: 1380, left: 6, img: '/images/r2.jpg' }, { id: 'r3', name: '湖景行政套房', bed: '2.0m大床', size: '88㎡', breakfast: '双早', price: 3280, left: 1, img: '/images/r3.jpg', max: 3 }] },
  { id: 'h2', name: '隐居·西溪湿地民宿', star: 0, score: 4.8, reviews: 1204, area: '西溪湿地', addr: '西湖区天目山路518号', dist: '距西溪湿地 0.8km', price: 698, img: '/images/h2.jpg', images: ['/images/h2.jpg', '/images/r3.jpg'], tags: ['庭院', '可做饭', '宠物友好'],
    facilities: ['免费WiFi', '私家庭院', '茶室', '厨房', '投影', '免费停车'],
    rooms: [{ id: 'r1', name: '竹林庭院大床房', bed: '1.8m大床', size: '32㎡', breakfast: '单早', price: 698, left: 2, img: '/images/r3.jpg' }, { id: 'r2', name: '复式亲子loft', bed: '1.8m+1.2m', size: '56㎡', breakfast: '双早', price: 1098, left: 1, img: '/images/r1.jpg', max: 4 }] },
  { id: 'h3', name: '全季酒店（杭州武林广场店）', star: 3, score: 4.7, reviews: 5630, area: '武林广场', addr: '拱墅区体育场路218号', dist: '距地铁武林广场站 200m', price: 389, img: '/images/h3.jpg', images: ['/images/h3.jpg', '/images/r2.jpg'], tags: ['近地铁', '自助洗衣'], hourly: true,
    facilities: ['免费WiFi', '自助洗衣', '行李寄存', '早餐', '24h前台'],
    rooms: [{ id: 'r1', name: '高级大床房', bed: '1.8m大床', size: '25㎡', breakfast: '无早', price: 389, left: 8, img: '/images/r2.jpg' }, { id: 'r2', name: '高级双床房', bed: '1.2m双床', size: '26㎡', breakfast: '单早', price: 429, left: 5, img: '/images/r1.jpg' }] },
  { id: 'h4', name: '杭州钱江新城万豪酒店', star: 5, score: 4.8, reviews: 4417, area: '钱江新城', addr: '上城区富春路28号', dist: '距来福士 0.5km', price: 1188, img: '/images/h4.jpg', images: ['/images/h4.jpg', '/images/r1.jpg', '/images/r3.jpg'], tags: ['江景', '行政酒廊', '室内泳池'],
    facilities: ['免费WiFi', '室内泳池', '健身房', '西餐厅', '行政酒廊', 'SPA', '洗衣服务', '24h前台'],
    rooms: [{ id: 'r1', name: '江景豪华大床房', bed: '2.0m大床', size: '48㎡', breakfast: '双早', price: 1188, left: 4, img: '/images/r1.jpg' }, { id: 'r2', name: '行政双床房', bed: '1.35m双床', size: '50㎡', breakfast: '双早', price: 1488, left: 2, img: '/images/r2.jpg' }] },
  { id: 'h5', name: '亚朵酒店（杭州湖滨银泰店）', star: 4, score: 4.8, reviews: 2981, area: '湖滨商圈', addr: '上城区东坡路36号', dist: '距湖滨步行街 150m', price: 628, img: '/images/h5.jpg', images: ['/images/h5.jpg', '/images/r2.jpg'], tags: ['近西湖', '24h阅读'], hourly: true,
    facilities: ['免费WiFi', '竹居书房', '健身房', '早餐', '洗衣房'],
    rooms: [{ id: 'r1', name: '几木大床房', bed: '1.8m大床', size: '30㎡', breakfast: '双早', price: 628, left: 5, img: '/images/r2.jpg' }, { id: 'r2', name: '湖景高级大床房', bed: '1.8m大床', size: '35㎡', breakfast: '双早', price: 798, left: 2, img: '/images/r1.jpg' }] },
  { id: 'h6', name: '汉庭酒店（杭州东站店）', star: 2, score: 4.5, reviews: 7215, area: '火车东站', addr: '上城区天城路68号', dist: '距杭州东站 400m', price: 219, img: '/images/h6.jpg', images: ['/images/h6.jpg', '/images/r3.jpg'], tags: ['近高铁', '性价比'], hourly: true,
    facilities: ['免费WiFi', '行李寄存', '24h前台'],
    rooms: [{ id: 'r1', name: '标准大床房', bed: '1.5m大床', size: '18㎡', breakfast: '无早', price: 219, left: 10, img: '/images/r3.jpg' }, { id: 'r2', name: '标准双床房', bed: '1.2m双床', size: '20㎡', breakfast: '无早', price: 239, left: 7, img: '/images/r2.jpg' }] }
];
const HOTELS = HZ.map(h => Object.assign({ city: '杭州' }, h)).concat(RAW.map(x => ({
  id: x[0], city: x[1], name: x[2], star: x[3], area: x[4], addr: x[5], dist: x[6], price: x[7], img: '/images/h' + x[8] + '.jpg',
  images: ['/images/h' + x[8] + '.jpg', '/images/r' + (x[8] % 3 + 1) + '.jpg'], tags: x[9], score: x[10], reviews: x[11], hourly: x[12],
  facilities: FAC[x[3]], rooms: roomsOf(x[3], x[7], x[8])
}))).map((h, i) => {
  const c = CITIES.find(x => x.name === h.city);
  const rooms = h.rooms.map(r => Object.assign({ max: 2 }, r));
  // 钟点房：4 小时，约当晚房价的 4 成
  if (h.hourly) rooms.push({ id: 'r9', name: '钟点房 · 4 小时', bed: rooms[0].bed, size: rooms[0].size, breakfast: '无早', price: Math.round(h.price * 0.4 / 10) * 10 - 2, left: 6, img: rooms[0].img, max: 2, hours: 4 });
  return Object.assign(h, {
    starText: STAR_TEXT[h.star], type: h.star ? '酒店' : '民宿', rooms,
    phone: c.code + '-' + String(87996688 - i * 113579).replace(/^(\d{4})(\d{4})$/, '$1 $2'),
    lat: Math.round((c.lat + ((i % 5) - 2) * 0.011) * 10000) / 10000,
    lng: Math.round((c.lng + ((i % 3) - 1) * 0.013) * 10000) / 10000
  });
});
// 点评：西湖国宾馆用原来那三条，其余按档次从池子里取
const POOL = {
  lux: [['旅行的小鹿', 5, '房间很大很干净，早餐种类丰富，前台还帮忙预约了行程，服务很贴心', ['服务热情', '早餐丰富']],
    ['Kevin_Z', 5, '带父母来住的，管家式服务很周到，停车方便，下次还来', ['服务热情']],
    ['一只柚子', 4, '景观房视野很好，床品舒服，就是周末人多，入住排了十分钟队', ['位置方便']]],
  mid: [['阿木木', 5, '出门就是地铁站，房间不大但很干净，性价比很高', ['位置方便', '性价比高']],
    ['晴天小猪', 5, '前台小哥很热情，免费升级了房型，洗衣房很方便', ['服务热情']],
    ['老周在路上', 3, '隔音一般，楼下有点吵，其他都挺好', ['干净卫生']]],
  eco: [['橘子汽水', 5, '离车站很近，赶早班车很方便，房间整洁', ['位置方便', '干净卫生']],
    ['老周在路上', 4, '价格实惠，该有的都有，热水很足', ['性价比高']]],
  bnb: [['旅行的小鹿', 5, '院子很美，老板很热情，推荐的小馆子都很好吃', ['服务热情', '安静舒适']],
    ['晴天小猪', 5, '房间布置很有设计感，早餐是老板娘亲手做的', ['早餐丰富']],
    ['一只柚子', 4, '位置稍微有点难找，但环境真的很安静，适合放空', ['安静舒适']]]
};
const H1_REVIEWS = [
  ['旅行的小鹿', 5, '推开窗就是西湖，早上看雾气慢慢散开特别美。前台小姐姐还帮忙预约了游船，服务很贴心', ['位置方便', '服务热情']],
  ['Kevin_Z', 5, '带父母来住的，房间很大很干净，早餐种类丰富，小笼包很好吃。停车方便，下次还来', ['早餐丰富', '干净卫生']],
  ['一只柚子', 4, '位置和景色没得说，国庆期间人多，入住排了十分钟队，其他都很满意', ['位置方便']]
];
function hotelOf(id) { return HOTELS.find(h => h.id === id) || HOTELS[0]; }
function cityOf(name) { return CITIES.find(c => c.name === name) || CITIES[0]; }
function seedReviews(h) {
  const src = h.id === 'h1' ? H1_REVIEWS : POOL[h.star === 5 ? 'lux' : h.star >= 3 ? 'mid' : h.star === 2 ? 'eco' : 'bnb'];
  return src.map((r, i) => ({ k: h.id + i, user: r[0], avatar: r[0].slice(0, 1), score: r[1], text: r[2], tags: r[3], room: h.rooms[i % h.rooms.length].name, ago: 7 + i * 9 }));
}
module.exports = { CITIES, STAR_TEXT, RATE_TAGS, ARRIVALS, firstArrival, HOUR_STARTS, NEEDS, SERVICE_PHONE, faqs, COUPONS, HOTELS, hotelOf, cityOf, seedReviews };
