// 城市与车次（本地生成，不联网）。同一组出发 / 到达 / 日期每次生成的车次完全一样
const CITIES = [
  ['B', ['北京', '保定', '蚌埠']],
  ['C', ['成都', '长沙', '重庆', '常州']],
  ['G', ['广州', '贵阳']],
  ['H', ['杭州', '合肥', '哈尔滨']],
  ['J', ['济南', '金华']],
  ['N', ['南京', '南昌', '宁波']],
  ['S', ['上海', '深圳', '苏州', '沈阳']],
  ['T', ['天津', '太原']],
  ['W', ['武汉', '无锡', '温州']],
  ['X', ['西安', '厦门', '徐州']],
  ['Z', ['郑州', '珠海']]
]
const HOT = ['北京', '上海', '广州', '深圳', '杭州', '南京', '武汉', '成都']
const ALL = [].concat.apply([], CITIES.map(g => g[1]))
// 席别：高铁 / 动车三种座，普速三种
const SEATS_G = [['商务座', 3.2], ['一等座', 1.6], ['二等座', 1]]
const SEATS_K = [['软卧', 2.1], ['硬卧', 1.4], ['硬座', 0.55]]
const PERIODS = [
  { key: 'p0', text: '00:00-06:00', from: 0, to: 360 },
  { key: 'p1', text: '06:00-12:00', from: 360, to: 720 },
  { key: 'p2', text: '12:00-18:00', from: 720, to: 1080 },
  { key: 'p3', text: '18:00-24:00', from: 1080, to: 1440 }
]
const SORTS = [
  { key: 'dep', text: '出发早' },
  { key: 'dur', text: '耗时短' },
  { key: 'price', text: '价格低' }
]
// 出发分钟、车型、耗时系数（相对基准时长）
const SLOTS = [
  [365, 'G', 1], [402, 'D', 1.25], [450, 'G', 1.05], [495, 'K', 2.6], [540, 'G', 0.95],
  [610, 'D', 1.3], [688, 'G', 1.1], [760, 'G', 1], [835, 'K', 2.4], [905, 'D', 1.2],
  [980, 'G', 1.15], [1065, 'G', 0.98], [1140, 'D', 1.35], [1230, 'K', 2.8], [1320, 'G', 1.08], [60, 'K', 3]
]
function hash(s) {
  let h = 7
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 100003
  return h
}
function pad(n) {
  return n < 10 ? '0' + n : '' + n
}
function hm(min) {
  min = ((min % 1440) + 1440) % 1440
  return pad(Math.floor(min / 60)) + ':' + pad(min % 60)
}
function durText(min) {
  return Math.floor(min / 60) + '时' + pad(min % 60) + '分'
}
function leftText(n) {
  return n === 0 ? '无' : n >= 20 ? '有' : n + '张'
}
// 某天的车次：基准时长、基准票价由城市对决定
function trains(from, to, date) {
  const h = hash(from + '-' + to)
  const base = 90 + (h % 240)
  const price = Math.round(base * 0.55 + 40)
  const dh = hash(date)
  return SLOTS.map((s, i) => {
    const dur = Math.round(base * s[2])
    const type = s[1]
    const seats = (type === 'K' ? SEATS_K : SEATS_G).map((x, j) => {
      const n = (h + dh + i * 17 + j * 29) % 60
      return { name: x[0], price: Math.round(price * x[1] * (type === 'D' ? 0.85 : type === 'K' ? 1 : 1)), left: n < 8 ? 0 : n - 7 }
    }).map(x => Object.assign(x, { leftText: leftText(x.left) }))
    const no = type + (type === 'K' ? 100 + (h + i * 37) % 800 : 1 + (h + i * 53) % 999)
    return {
      no, type, from, to, date,
      dep: s[0], depText: hm(s[0]), arrText: hm(s[0] + dur), nextDay: s[0] + dur >= 1440,
      dur, durText: durText(dur), seats,
      minPrice: Math.min.apply(null, seats.map(x => x.price))
    }
  })
}
function isHighSpeed(t) {
  return t.type === 'G' || t.type === 'D'
}
// 日期工具：本地时区的 YYYY-MM-DD
function fmtDate(d) {
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate())
}
function parse(s) {
  const p = s.split('-').map(Number)
  return new Date(p[0], p[1] - 1, p[2])
}
function addDays(s, n) {
  const d = parse(s)
  d.setDate(d.getDate() + n)
  return fmtDate(d)
}
function today() {
  return fmtDate(new Date())
}
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
function dateLabel(s) {
  const p = s.split('-')
  const t = today()
  const tag = s === t ? '今天' : s === addDays(t, 1) ? '明天' : s === addDays(t, 2) ? '后天' : WEEK[parse(s).getDay()]
  return Number(p[1]) + '月' + Number(p[2]) + '日 ' + tag
}
module.exports = { CITIES, HOT, ALL, PERIODS, SORTS, trains, isHighSpeed, today, addDays, dateLabel, hm }
