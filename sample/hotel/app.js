const HOTELS = [
  {
    id: 'h1', name: '杭州西湖国宾馆·湖景楼', star: 5, starText: '豪华型', score: 4.9, reviews: 3826, area: '西湖风景区', addr: '西湖区杨公堤18号', dist: '距西湖 0.3km',
    price: 1680, img: '/images/h1.jpg', images: ['/images/h1.jpg', '/images/r1.jpg', '/images/r2.jpg'], tags: ['湖景房', '免费停车', '含早'], type: '酒店',
    facilities: ['免费WiFi', '室外泳池', '健身房', '中餐厅', '行李寄存', '接机服务', '会议厅', '24h前台'],
    rooms: [
      { id: 'r1', name: '湖景豪华大床房', bed: '1.8m大床', size: '45㎡', breakfast: '双早', price: 1680, left: 3, img: '/images/r1.jpg' },
      { id: 'r2', name: '园景高级双床房', bed: '1.2m双床', size: '40㎡', breakfast: '双早', price: 1380, left: 6, img: '/images/r2.jpg' },
      { id: 'r3', name: '湖景行政套房', bed: '2.0m大床', size: '88㎡', breakfast: '双早', price: 3280, left: 1, img: '/images/r3.jpg' }
    ]
  },
  {
    id: 'h2', name: '隐居·西溪湿地民宿', star: 0, starText: '特色民宿', score: 4.8, reviews: 1204, area: '西溪湿地', addr: '西湖区天目山路518号', dist: '距西溪湿地 0.8km',
    price: 698, img: '/images/h2.jpg', images: ['/images/h2.jpg', '/images/r3.jpg'], tags: ['庭院', '可做饭', '宠物友好'], type: '民宿',
    facilities: ['免费WiFi', '私家庭院', '茶室', '厨房', '投影', '免费停车'],
    rooms: [
      { id: 'r1', name: '竹林庭院大床房', bed: '1.8m大床', size: '32㎡', breakfast: '单早', price: 698, left: 2, img: '/images/r3.jpg' },
      { id: 'r2', name: '复式亲子loft', bed: '1.8m+1.2m', size: '56㎡', breakfast: '双早', price: 1098, left: 1, img: '/images/r1.jpg' }
    ]
  },
  {
    id: 'h3', name: '全季酒店（杭州武林广场店）', star: 3, starText: '舒适型', score: 4.7, reviews: 5630, area: '武林广场', addr: '拱墅区体育场路218号', dist: '距地铁武林广场站 200m',
    price: 389, img: '/images/h3.jpg', images: ['/images/h3.jpg', '/images/r2.jpg'], tags: ['近地铁', '自助洗衣'], type: '酒店',
    facilities: ['免费WiFi', '自助洗衣', '行李寄存', '早餐', '24h前台'],
    rooms: [
      { id: 'r1', name: '高级大床房', bed: '1.8m大床', size: '25㎡', breakfast: '无早', price: 389, left: 8, img: '/images/r2.jpg' },
      { id: 'r2', name: '高级双床房', bed: '1.2m双床', size: '26㎡', breakfast: '单早', price: 429, left: 5, img: '/images/r1.jpg' }
    ]
  },
  {
    id: 'h4', name: '杭州钱江新城万豪酒店', star: 5, starText: '豪华型', score: 4.8, reviews: 4417, area: '钱江新城', addr: '上城区富春路28号', dist: '距来福士 0.5km',
    price: 1188, img: '/images/h4.jpg', images: ['/images/h4.jpg', '/images/r1.jpg', '/images/r3.jpg'], tags: ['江景', '行政酒廊', '室内泳池'], type: '酒店',
    facilities: ['免费WiFi', '室内泳池', '健身房', '西餐厅', '行政酒廊', 'SPA', '洗衣服务', '24h前台'],
    rooms: [
      { id: 'r1', name: '江景豪华大床房', bed: '2.0m大床', size: '48㎡', breakfast: '双早', price: 1188, left: 4, img: '/images/r1.jpg' },
      { id: 'r2', name: '行政双床房', bed: '1.35m双床', size: '50㎡', breakfast: '双早', price: 1488, left: 2, img: '/images/r2.jpg' }
    ]
  },
  {
    id: 'h5', name: '亚朵酒店（杭州湖滨银泰店）', star: 4, starText: '高档型', score: 4.8, reviews: 2981, area: '湖滨商圈', addr: '上城区东坡路36号', dist: '距湖滨步行街 150m',
    price: 628, img: '/images/h5.jpg', images: ['/images/h5.jpg', '/images/r2.jpg'], tags: ['近西湖', '24h阅读'], type: '酒店',
    facilities: ['免费WiFi', '竹居书房', '健身房', '早餐', '洗衣房'],
    rooms: [
      { id: 'r1', name: '几木大床房', bed: '1.8m大床', size: '30㎡', breakfast: '双早', price: 628, left: 5, img: '/images/r2.jpg' },
      { id: 'r2', name: '湖景高级大床房', bed: '1.8m大床', size: '35㎡', breakfast: '双早', price: 798, left: 2, img: '/images/r1.jpg' }
    ]
  },
  {
    id: 'h6', name: '汉庭酒店（杭州东站店）', star: 2, starText: '经济型', score: 4.5, reviews: 7215, area: '火车东站', addr: '上城区天城路68号', dist: '距杭州东站 400m',
    price: 219, img: '/images/h6.jpg', images: ['/images/h6.jpg', '/images/r3.jpg'], tags: ['近高铁', '性价比'], type: '酒店',
    facilities: ['免费WiFi', '行李寄存', '24h前台'],
    rooms: [
      { id: 'r1', name: '标准大床房', bed: '1.5m大床', size: '18㎡', breakfast: '无早', price: 219, left: 10, img: '/images/r3.jpg' },
      { id: 'r2', name: '标准双床房', bed: '1.2m双床', size: '20㎡', breakfast: '无早', price: 239, left: 7, img: '/images/r2.jpg' }
    ]
  }
];

const REVIEWS = [
  { user: '旅行的小鹿', avatar: '鹿', score: 5, date: '2026-10-19', room: '湖景豪华大床房', text: '推开窗就是西湖，早上看雾气慢慢散开特别美。前台小姐姐还帮忙预约了游船，服务很贴心。' },
  { user: 'Kevin_Z', avatar: 'K', score: 5, date: '2026-10-12', room: '园景高级双床房', text: '带父母来住的，房间很大很干净，早餐种类丰富，小笼包很好吃。停车方便，下次还来。' },
  { user: '一只柚子', avatar: '柚', score: 4, date: '2026-10-03', room: '湖景豪华大床房', text: '位置和景色没得说，国庆期间人多，入住排了十分钟队，其他都很满意。' }
];

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function fmt(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function parse(s) { const p = s.split('-'); return new Date(+p[0], +p[1] - 1, +p[2]); }
function short(s) { const d = parse(s); return (d.getMonth() + 1) + '月' + d.getDate() + '日'; }
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
function week(s) { return WEEK[parse(s).getDay()]; }
function nights(a, b) { return Math.max(1, Math.round((parse(b) - parse(a)) / 86400000)); }

App({
  HOTELS, REVIEWS,
  util: { fmt, parse, short, week, nights },
  getHotel(id) { return HOTELS.find(h => h.id === id) || HOTELS[0]; },
  getSearch() {
    const today = new Date();
    const tomorrow = new Date(today.getTime() + 86400000);
    const s = wx.getStorageSync('hotel_search') || {};
    const def = { city: '杭州', checkin: fmt(today), checkout: fmt(tomorrow), keyword: '', price: 0, star: 0 };
    const out = Object.assign(def, s);
    if (out.checkin < fmt(today)) { out.checkin = fmt(today); out.checkout = fmt(tomorrow); }
    return out;
  },
  saveSearch(s) { wx.setStorageSync('hotel_search', s); },
  getOrders() { return wx.getStorageSync('hotel_orders') || []; },
  saveOrders(o) { wx.setStorageSync('hotel_orders', o); }
});
