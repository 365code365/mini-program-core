// 房源数据（杭州，虚构）。type: new 新房 / second 二手 / rent 租房
const AGENTS = {
  a1: { name: '沈嘉怡', store: '安居找房 · 滨江星光店', years: 6, deals: 238, rate: '99%', initial: '沈', color: '#ff8a3d', phone: '138 0571 2368' },
  a2: { name: '陆明轩', store: '安居找房 · 西溪印象城店', years: 4, deals: 156, rate: '98%', initial: '陆', color: '#3a86ff', phone: '139 5810 7712' },
  a3: { name: '许婧', store: '安居找房 · 未来科技城店', years: 8, deals: 412, rate: '100%', initial: '许', color: '#19a974', phone: '137 7745 0936' }
}

const HOUSES = [
  {
    id: 'h1', lat: 30.2105, lng: 120.2121, addr: '滨江区江陵路 1688 号', type: 'second', title: '龙湖春江天玺 南北通透三房 满五唯一', community: '龙湖春江天玺', district: '滨江', area: 128, layout: '3室2厅2卫', rooms: 3,
    face: '南北', floor: '中楼层/32层', year: 2019, deco: '精装', price: 568, unit: 44375, tags: ['满五唯一', '近地铁', '随时看房'],
    images: ['h1_living', 'p3', 'bed1'], agent: 'a1', lift: '有', heat: '地暖', subway: '距 6 号线江陵路站 450 米', views: 1287, follow: 86
  },
  {
    id: 'h2', lat: 30.2652, lng: 120.0716, addr: '西湖区西溪路 956 号', type: 'second', title: '西溪诚园 精装两房 业主诚心出售', community: '西溪诚园', district: '西湖', area: 89, layout: '2室1厅1卫', rooms: 2,
    face: '南', floor: '高楼层/18层', year: 2012, deco: '精装', price: 412, unit: 46292, tags: ['满二', '学区房', '南向'],
    images: ['h2_living', 'p2', 'bed2'], agent: 'a2', lift: '有', heat: '无', subway: '距 19 号线西溪湿地南站 800 米', views: 932, follow: 51
  },
  {
    id: 'h3', lat: 30.2868, lng: 119.9891, addr: '余杭区良睦路 1399 号', type: 'new', title: '万科未来天空之城 四房 品质大平层', community: '万科未来天空之城', district: '余杭', area: 143, layout: '4室2厅2卫', rooms: 4,
    face: '南北', floor: '在售 8~26 层', year: 2026, deco: '精装', price: 543, unit: 38000, tags: ['在售', '品牌房企', '低密社区'],
    images: ['h3_living', 'p4', 'bed3'], agent: 'a3', lift: '有', heat: '地暖', subway: '距 5 号线良睦路站 1.2 公里', views: 3561, follow: 420
  },
  {
    id: 'h4', lat: 30.3247, lng: 120.1466, addr: '拱墅区丽水路 288 号', type: 'new', title: '运河宸园 三房 一线运河景观', community: '运河宸园', district: '拱墅', area: 115, layout: '3室2厅2卫', rooms: 3,
    face: '南', floor: '在售 3~17 层', year: 2025, deco: '精装', price: 489, unit: 42500, tags: ['即将交付', '运河景观', '商业配套'],
    images: ['h4_living', 'p3', 'bed1'], agent: 'a2', lift: '有', heat: '地暖', subway: '距 5 号线大运河站 600 米', views: 2104, follow: 233
  },
  {
    id: 'h5', lat: 30.2486, lng: 120.2128, addr: '上城区新业路 228 号', type: 'rent', title: '钱江新城 来福士公寓 整租一室 拎包入住', community: '来福士中心', district: '上城', area: 52, layout: '1室1厅1卫', rooms: 1,
    face: '东南', floor: '高楼层/58层', year: 2017, deco: '豪装', price: 6800, unit: 0, tags: ['整租', '押一付三', '近地铁', '可短租'],
    images: ['h5_living', 'p1', 'bed2'], agent: 'a1', lift: '有', heat: '中央空调', subway: '距 4 号线江锦路站 200 米', views: 645, follow: 38
  },
  {
    id: 'h6', lat: 30.2075, lng: 120.201, addr: '滨江区滨盛路 1508 号', type: 'rent', title: '江南名府 精装两房 南向带阳台', community: '江南名府', district: '滨江', area: 76, layout: '2室1厅1卫', rooms: 2,
    face: '南', floor: '中楼层/24层', year: 2015, deco: '精装', price: 4500, unit: 0, tags: ['整租', '月付', '新上'],
    images: ['h6_living', 'p2', 'bed3'], agent: 'a3', lift: '有', heat: '无', subway: '距 1 号线滨和路站 700 米', views: 418, follow: 22
  }
]

// 各区二手房参考均价（元/㎡）与环比（%），查房价 / 地图找房 / 卖房估价共用
const DISTRICTS = [
  { name: '上城', avg: 52680, mom: -0.4, deals: 812, x: 62, y: 46 },
  { name: '西湖', avg: 48930, mom: -0.8, deals: 976, x: 26, y: 40 },
  { name: '滨江', avg: 47210, mom: 0.3, deals: 1104, x: 58, y: 70 },
  { name: '拱墅', avg: 41860, mom: -0.6, deals: 893, x: 46, y: 18 },
  { name: '余杭', avg: 31570, mom: -1.1, deals: 1429, x: 12, y: 16 }
]
const TYPE_TEXT = { new: '新房', second: '二手房', rent: '租房' }

function priceText(h) {
  if (h.type === 'rent') return { main: h.price, unit: '元/月', sub: '' }
  if (h.type === 'new') return { main: h.unit, unit: '元/㎡', sub: '总价约 ' + h.price + ' 万' }
  return { main: h.price, unit: '万', sub: h.unit + ' 元/㎡' }
}

function decorate(h) {
  const p = priceText(h)
  return Object.assign({}, h, {
    cover: '/images/' + h.images[0] + '.png',
    priceMain: p.main, priceUnit: p.unit, priceSub: p.sub,
    typeText: TYPE_TEXT[h.type],
    brief: h.layout + ' | ' + h.area + '㎡ | ' + h.face + ' | ' + h.district
  })
}

function all() {
  return HOUSES.map(decorate)
}

function get(id) {
  const h = HOUSES.find(i => i.id === id) || HOUSES[0]
  return decorate(h)
}
// 找不到返回 null（预约记录里的房源下架了之类）
function find(id) {
  const h = HOUSES.find(i => i.id === id)
  return h ? decorate(h) : null
}


// 筛选条件（列表页、订阅共用）。价格档：二手按总价（万）、新房按单价（元/㎡）、租房按月租（元）
const PRICE = {
  second: [{ text: '不限' }, { text: '400 万以下', max: 400 }, { text: '400-500 万', min: 400, max: 500 }, { text: '500 万以上', min: 500 }],
  new: [{ text: '不限' }, { text: '4 万/㎡ 以下', umax: 40000 }, { text: '4 万/㎡ 以上', umin: 40000 }],
  rent: [{ text: '不限' }, { text: '5000 元以下', max: 5000 }, { text: '5000 元以上', min: 5000 }]
}
const DISTRICT_NAMES = ['不限'].concat(DISTRICTS.map(d => d.name))
const LAYOUTS = ['不限', '一室', '两室', '三室', '四室及以上']
// c: { type, kw, district（'不限' 或区名）, pIndex, lIndex }
function filter(c) {
  const pr = (PRICE[c.type] || [])[c.pIndex || 0] || {}
  const kw = c.kw || ''
  const li = c.lIndex || 0
  return all().filter(h => {
    if (h.type !== c.type) return false
    if (kw && (h.title + h.community + h.district + h.subway).indexOf(kw) < 0) return false
    if (c.district && c.district !== '不限' && h.district !== c.district) return false
    if (pr.min && h.price < pr.min) return false
    if (pr.max && h.price >= pr.max) return false
    if (pr.umin && h.unit < pr.umin) return false
    if (pr.umax && h.unit >= pr.umax) return false
    if (li && li < 4 && h.rooms !== li) return false
    if (li === 4 && h.rooms < 4) return false
    return true
  })
}
function condText(c) {
  const parts = [TYPE_TEXT[c.type]]
  if (c.district && c.district !== '不限') parts.push(c.district)
  if (c.lIndex) parts.push(LAYOUTS[c.lIndex])
  if (c.pIndex) parts.push(PRICE[c.type][c.pIndex].text)
  return parts.join(' · ')
}
// 卖房估价：区域均价 × 面积，楼层 / 装修 / 房龄修正。返回 { unit 元/㎡, total 万, low, high }
const FLOORS = ['低楼层', '中楼层', '高楼层']
const DECOS = ['毛坯', '简装', '精装', '豪装']
function estimate(e) {
  const d = DISTRICTS.find(x => x.name === e.district) || DISTRICTS[0]
  let k = 1 + [-0.03, 0, 0.02][e.floor || 0] + [-0.06, -0.02, 0, 0.04][e.deco || 0]
  const age = Math.max(0, 2026 - (e.year || 2015))
  k -= Math.min(0.15, age * 0.006)
  const unit = Math.round(d.avg * k / 10) * 10
  const total = Math.round(unit * e.area / 10000)
  return { unit, total, low: Math.round(total * 0.96), high: Math.round(total * 1.04) }
}
module.exports = { AGENTS, DISTRICTS, DISTRICT_NAMES, TYPE_TEXT, PRICE, LAYOUTS, FLOORS, DECOS, all, get, find, filter, condText, estimate }
