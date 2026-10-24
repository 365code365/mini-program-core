// 房源数据（杭州，虚构）。type: new 新房 / second 二手 / rent 租房
const AGENTS = {
  a1: { name: '沈嘉怡', store: '安居找房 · 滨江星光店', years: 6, deals: 238, rate: '99%', initial: '沈', color: '#ff8a3d' },
  a2: { name: '陆明轩', store: '安居找房 · 西溪印象城店', years: 4, deals: 156, rate: '98%', initial: '陆', color: '#3a86ff' },
  a3: { name: '许婧', store: '安居找房 · 未来科技城店', years: 8, deals: 412, rate: '100%', initial: '许', color: '#19a974' }
}

const HOUSES = [
  {
    id: 'h1', type: 'second', title: '龙湖春江天玺 南北通透三房 满五唯一', community: '龙湖春江天玺', district: '滨江', area: 128, layout: '3室2厅2卫', rooms: 3,
    face: '南北', floor: '中楼层/32层', year: 2019, deco: '精装', price: 568, unit: 44375, tags: ['满五唯一', '近地铁', '随时看房'],
    images: ['h1_living', 'p3', 'bed1'], agent: 'a1', lift: '有', heat: '地暖', subway: '距 6 号线江陵路站 450 米', views: 1287, follow: 86
  },
  {
    id: 'h2', type: 'second', title: '西溪诚园 精装两房 业主诚心出售', community: '西溪诚园', district: '西湖', area: 89, layout: '2室1厅1卫', rooms: 2,
    face: '南', floor: '高楼层/18层', year: 2012, deco: '精装', price: 412, unit: 46292, tags: ['满二', '学区房', '南向'],
    images: ['h2_living', 'p2', 'bed2'], agent: 'a2', lift: '有', heat: '无', subway: '距 19 号线西溪湿地南站 800 米', views: 932, follow: 51
  },
  {
    id: 'h3', type: 'new', title: '万科未来天空之城 四房 品质大平层', community: '万科未来天空之城', district: '余杭', area: 143, layout: '4室2厅2卫', rooms: 4,
    face: '南北', floor: '在售 8~26 层', year: 2026, deco: '精装', price: 543, unit: 38000, tags: ['在售', '品牌房企', '低密社区'],
    images: ['h3_living', 'p4', 'bed3'], agent: 'a3', lift: '有', heat: '地暖', subway: '距 5 号线良睦路站 1.2 公里', views: 3561, follow: 420
  },
  {
    id: 'h4', type: 'new', title: '运河宸园 三房 一线运河景观', community: '运河宸园', district: '拱墅', area: 115, layout: '3室2厅2卫', rooms: 3,
    face: '南', floor: '在售 3~17 层', year: 2025, deco: '精装', price: 489, unit: 42500, tags: ['即将交付', '运河景观', '商业配套'],
    images: ['h4_living', 'p3', 'bed1'], agent: 'a2', lift: '有', heat: '地暖', subway: '距 5 号线大运河站 600 米', views: 2104, follow: 233
  },
  {
    id: 'h5', type: 'rent', title: '钱江新城 来福士公寓 整租一室 拎包入住', community: '来福士中心', district: '上城', area: 52, layout: '1室1厅1卫', rooms: 1,
    face: '东南', floor: '高楼层/58层', year: 2017, deco: '豪装', price: 6800, unit: 0, tags: ['整租', '押一付三', '近地铁', '可短租'],
    images: ['h5_living', 'p1', 'bed2'], agent: 'a1', lift: '有', heat: '中央空调', subway: '距 4 号线江锦路站 200 米', views: 645, follow: 38
  },
  {
    id: 'h6', type: 'rent', title: '江南名府 精装两房 南向带阳台', community: '江南名府', district: '滨江', area: 76, layout: '2室1厅1卫', rooms: 2,
    face: '南', floor: '中楼层/24层', year: 2015, deco: '精装', price: 4500, unit: 0, tags: ['整租', '月付', '新上'],
    images: ['h6_living', 'p2', 'bed3'], agent: 'a3', lift: '有', heat: '无', subway: '距 1 号线滨和路站 700 米', views: 418, follow: 22
  }
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

function getFavs() {
  return wx.getStorageSync('house_favs') || []
}

function toggleFav(id) {
  const favs = getFavs()
  const i = favs.indexOf(id)
  if (i >= 0) favs.splice(i, 1)
  else favs.unshift(id)
  wx.setStorageSync('house_favs', favs)
  return i < 0
}

function getBookings() {
  return wx.getStorageSync('house_bookings') || []
}

function addBooking(b) {
  const list = getBookings()
  list.unshift(b)
  wx.setStorageSync('house_bookings', list)
}

module.exports = { AGENTS, TYPE_TEXT, all, get, getFavs, toggleFav, getBookings, addBooking }
