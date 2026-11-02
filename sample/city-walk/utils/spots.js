// 北京城区的地点（坐标 GCJ-02，与微信 map 的默认坐标系一致；票价、开放时间按各馆官方公布的常规标准）
const CATS = [
  { key: 'all', name: '全部' },
  { key: 'sight', name: '名胜', color: '#e8590c' },
  { key: 'museum', name: '展馆', color: '#1c7ed6' },
  { key: 'street', name: '街区', color: '#2f9e44' },
  { key: 'park', name: '公园', color: '#0ca678' }
];

const SPOTS = [
  {
    id: 1, cat: 'sight', name: '故宫博物院', lat: 39.916345, lng: 116.397155, cover: '/images/spots/gugong.jpg',
    price: '旺季 60 元 / 淡季 40 元', hours: '08:30-17:00（周一闭馆）', duration: '3-4 小时', address: '东城区景山前街 4 号',
    intro: '明清两代的皇家宫殿，现存古建筑 9000 余间。午门进、神武门出，中轴线三大殿与东西六宫都值得一看。',
    tips: ['实名预约，开放前 7 天 20:00 放票', '只能从午门进入', '珍宝馆、钟表馆另收 10 元'], live: true
  },
  {
    id: 2, cat: 'sight', name: '天安门广场', lat: 39.903179, lng: 116.397755, cover: '/images/spots/tiananmen.jpg',
    price: '免费（需预约）', hours: '05:00-22:00', duration: '1 小时', address: '东城区东长安街',
    intro: '世界上最大的城市广场之一，人民英雄纪念碑、国家博物馆、人民大会堂环绕四周。升旗仪式时间随日出变化。',
    tips: ['升旗前约 1 小时开始安检', '广场内不能放风筝、无人机'], live: true
  },
  {
    id: 3, cat: 'museum', name: '中国国家博物馆', lat: 39.905108, lng: 116.401276, cover: '/images/spots/guobo.jpg',
    price: '免费（需预约）', hours: '09:00-17:00（周一闭馆）', duration: '3 小时', address: '东城区东长安街 16 号',
    intro: '常设展「古代中国」从远古讲到明清，后母戊鼎、四羊方尊等国宝都在这里。',
    tips: ['提前 7 天预约', '西门安检排队较短'], live: false
  },
  {
    id: 4, cat: 'park', name: '景山公园', lat: 39.923783, lng: 116.396900, cover: '/images/spots/jingshan.jpg',
    price: '2 元', hours: '06:30-21:00', duration: '1 小时', address: '西城区景山西街 44 号',
    intro: '登万春亭能俯瞰故宫全景，傍晚看金色的琉璃瓦屋顶最好看。',
    tips: ['从北门进可少走回头路', '日落前 40 分钟上山'], live: true
  },
  {
    id: 5, cat: 'street', name: '南锣鼓巷', lat: 39.937238, lng: 116.403116, cover: '/images/spots/nanluo.jpg',
    price: '免费', hours: '全天', duration: '1.5 小时', address: '东城区南锣鼓巷',
    intro: '元代胡同肌理保存最完整的街区之一，主街两侧的东棉花胡同、帽儿胡同更安静。',
    tips: ['主街周末人多，可以绕到两侧胡同', '可以骑共享单车串到什刹海'], live: false
  },
  {
    id: 6, cat: 'park', name: '北海公园', lat: 39.925310, lng: 116.389720, cover: '/images/spots/beihai.jpg',
    price: '旺季 10 元', hours: '06:30-21:00', duration: '2 小时', address: '西城区文津街 1 号',
    intro: '辽金时期起就是皇家园林，琼华岛上的白塔是北京城的标志之一，湖上可以划船。',
    tips: ['白塔需另购联票', '东门离故宫北门最近'], live: false
  },
  {
    id: 7, cat: 'street', name: '前门大街', lat: 39.896070, lng: 116.398150, cover: '/images/spots/qianmen.jpg',
    price: '免费', hours: '全天', duration: '1.5 小时', address: '东城区前门大街',
    intro: '老字号聚集的步行街，全聚德、都一处、大北照相都在这条街上；大栅栏就在西侧。',
    tips: ['铛铛车 20 元一程', '晚上亮灯后更有味道'], live: false
  },
  {
    id: 8, cat: 'museum', name: '首都博物馆', lat: 39.905630, lng: 116.342280, cover: '/images/spots/shoubo.jpg',
    price: '免费（需预约）', hours: '09:00-17:00（周一闭馆）', duration: '2.5 小时', address: '西城区复兴门外大街 16 号',
    intro: '讲北京城的历史与民俗，「京城旧事」展厅复原了胡同、四合院和老北京婚俗。',
    tips: ['地铁 1 号线木樨地站 C1 口', '馆内有餐厅'], live: false
  }
];

const ICONS = { sight: '/images/pin/sight.png', museum: '/images/pin/museum.png', street: '/images/pin/street.png', park: '/images/pin/park.png' };

function byId(id) {
  return SPOTS.find((s) => s.id === Number(id));
}

/// 地图标记（官方 markers 字段）。选中的那个常显气泡
function markers(list, selected) {
  return list.map((s) => ({
    id: s.id, latitude: s.lat, longitude: s.lng, iconPath: ICONS[s.cat], width: 30, height: 38, title: s.name,
    callout: { content: s.name, display: s.id === selected ? 'ALWAYS' : 'BYCLICK', padding: 6, borderRadius: 6, fontSize: 12,
      bgColor: s.id === selected ? '#1c3d5a' : '#ffffff', color: s.id === selected ? '#ffffff' : '#1c3d5a' }
  }));
}

/// 两点间的球面距离（米）
function distance(a, b) {
  const R = 6371000;
  const r = (d) => d * Math.PI / 180;
  const dLat = r(b.lat - a.lat), dLng = r(b.lng - a.lng);
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(r(a.lat)) * Math.cos(r(b.lat)) * Math.sin(dLng / 2) ** 2;
  return 2 * R * Math.asin(Math.sqrt(h));
}

function fmtDistance(m) {
  return m >= 1000 ? (m / 1000).toFixed(1) + ' 公里' : Math.round(m) + ' 米';
}

module.exports = { CATS, SPOTS, ICONS, byId, markers, distance, fmtDistance };
