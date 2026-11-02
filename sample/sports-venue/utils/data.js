// 场馆与运动项目（场馆名为虚构，地址、坐标在杭州真实区域内）。价格为每片场地每小时。
const SPORTS = [
  { id: 'badminton', name: '羽毛球', icon: '/images/sport/badminton.png' },
  { id: 'basketball', name: '篮球', icon: '/images/sport/basketball.png' },
  { id: 'tennis', name: '网球', icon: '/images/sport/tennis.png' },
  { id: 'football', name: '足球', icon: '/images/sport/football.png' },
  { id: 'pingpong', name: '乒乓球', icon: '/images/sport/pingpong.png' },
  { id: 'pickle', name: '匹克球', icon: '/images/sport/pickle.png' },
  { id: 'swim', name: '游泳', icon: '/images/sport/swim.png' },
  { id: 'yoga', name: '瑜伽', icon: '/images/sport/yoga.png' }
];

// 默认定位：杭州市西湖区文三路（定位失败时用）
const DEFAULT_LOC = { latitude: 30.2779, longitude: 120.1397, name: '杭州 · 西湖区' };

const VENUES = [
  {
    id: 'v1', sport: 'badminton', name: '西溪文体中心羽毛球馆', photo: '/images/venue/v1.jpg',
    addr: '杭州市西湖区天目山路398号 西溪文体中心3楼', lat: 30.2764, lng: 120.1215, phone: '0571-87651230',
    open: '08:00-22:00', rating: 4.8, courts: 8, courtName: '号场', price: 60, peak: 90,
    facilities: ['木地板 + 专业地胶', '无柱大跨度', '免费停车 2 小时', '淋浴间', '器材租借', '空调'],
    notice: '请穿着运动鞋入场；场地按小时预订，可提前 10 分钟入场热身；开场前 2 小时以上可免费取消。'
  },
  {
    id: 'v2', sport: 'basketball', name: '城西动力篮球公园', photo: '/images/venue/v2.jpg',
    addr: '杭州市拱墅区丰潭路380号 城西银泰城北侧', lat: 30.2958, lng: 120.1123, phone: '0571-88213366',
    open: '09:00-22:00', rating: 4.6, courts: 4, courtName: '号全场', price: 280, peak: 380,
    facilities: ['室内悬浮地板', 'FIBA 标准篮架', '电子记分牌', '更衣柜', '饮料售卖'],
    notice: '全场按小时预订，可容纳 10~12 人；禁止穿黑底鞋；开场前 2 小时以上可免费取消。'
  },
  {
    id: 'v3', sport: 'tennis', name: '滨江奥体网球中心', photo: '/images/venue/v3.jpg',
    addr: '杭州市滨江区飞虹路3号 奥体中心南区', lat: 30.2307, lng: 120.2295, phone: '0571-86778899',
    open: '07:00-22:00', rating: 4.9, courts: 6, courtName: '号场', price: 120, peak: 180,
    facilities: ['硬地球场', '夜场灯光', '教练陪练', '发球机租赁', '停车场'],
    notice: '雨天室外场地顺延或全额退款；夜场 18:00 后开灯；开场前 2 小时以上可免费取消。'
  },
  {
    id: 'v4', sport: 'football', name: '之江足球公园', photo: '/images/venue/v4.jpg',
    addr: '杭州市西湖区之江路1288号', lat: 30.1963, lng: 120.1371, phone: '0571-85120077',
    open: '08:00-22:00', rating: 4.7, courts: 4, courtName: '号五人制', price: 300, peak: 450,
    facilities: ['人造草坪', '五人制 / 七人制', '夜场灯光', '对抗背心', '淋浴间'],
    notice: '请穿碎钉或平底足球鞋，禁止长钉；开场前 2 小时以上可免费取消。'
  },
  {
    id: 'v5', sport: 'pingpong', name: '文三乒乓球俱乐部', photo: '/images/venue/v5.jpg',
    addr: '杭州市西湖区文三路259号 昌地火炬大厦裙楼2楼', lat: 30.2783, lng: 120.1419, phone: '0571-88907720',
    open: '09:00-22:00', rating: 4.5, courts: 8, courtName: '号台', price: 30, peak: 45,
    facilities: ['红双喜彩虹球台', '专业挡板', '球拍租借', '空调'],
    notice: '按球台计费，每台限 4 人；开场前 2 小时以上可免费取消。'
  },
  {
    id: 'v6', sport: 'pickle', name: '钱江新城匹克球馆', photo: '/images/venue/v6.jpg',
    addr: '杭州市上城区富春路290号 钱江国际广场B1', lat: 30.2472, lng: 120.2113, phone: '0571-86021515',
    open: '08:00-22:00', rating: 4.7, courts: 4, courtName: '号场', price: 80, peak: 120,
    facilities: ['标准匹克球场', '球拍与球免费借用', '新手教学', '休息区'],
    notice: '新手每周六上午有免费体验课；开场前 2 小时以上可免费取消。'
  }
];

// 会员卡套餐
const PLANS = [
  { id: 'r500', kind: 'stored', name: '储值 500', pay: 500, give: 50, desc: '到账 550 元，全场馆通用' },
  { id: 'r1000', kind: 'stored', name: '储值 1000', pay: 1000, give: 150, desc: '到账 1150 元，全场馆通用', hot: true },
  { id: 'r2000', kind: 'stored', name: '储值 2000', pay: 2000, give: 400, desc: '到账 2400 元，全场馆通用' },
  { id: 'c-bad10', kind: 'times', sport: 'badminton', name: '羽毛球 10 次卡', pay: 600, times: 10, desc: '每次抵 1 小时场地（含高峰），有效期 180 天' },
  { id: 'c-pp20', kind: 'times', sport: 'pingpong', name: '乒乓球 20 次卡', pay: 560, times: 20, desc: '每次抵 1 小时球台，有效期 180 天' },
  { id: 'c-ten5', kind: 'times', sport: 'tennis', name: '网球 5 次卡', pay: 700, times: 5, desc: '每次抵 1 小时场地（含高峰），有效期 90 天' }
];

const byId = {};
VENUES.forEach((v) => { byId[v.id] = v; });
function getVenue(id) { return byId[id] || null; }
function sportName(id) { const s = SPORTS.find((x) => x.id === id); return s ? s.name : ''; }

// 两点距离（km），haversine
function distance(a, b) {
  const R = 6371, rad = Math.PI / 180;
  const dLat = (b.latitude - a.latitude) * rad, dLng = (b.longitude - a.longitude) * rad;
  const x = Math.sin(dLat / 2) ** 2 + Math.cos(a.latitude * rad) * Math.cos(b.latitude * rad) * Math.sin(dLng / 2) ** 2;
  return 2 * R * Math.asin(Math.sqrt(x));
}

module.exports = { SPORTS, VENUES, PLANS, DEFAULT_LOC, getVenue, sportName, distance };
