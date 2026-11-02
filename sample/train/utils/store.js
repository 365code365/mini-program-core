// 乘车人、订单、查询条件：落在本地 storage（tr_passengers / tr_orders / tr_query）
// 订单 status：paid 已出票 / waiting 候补中 / cancelled 已退票或已取消候补
const D = require('./data.js')
const SEED_PASSENGERS = [
  { id: 1, name: '李明', idcard: '330106199003071236', phone: '13812345678', self: true }
]
const HOUR = 3600000
// 候补：提交后 1 分钟内有人退票就自动出票（真 12306 由后台排队兑现，这里按时间推进）
const FULFILL_MS = 60000

function round2(n) {
  return Math.round(n * 100) / 100
}
function hash(s) {
  let h = 7
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 100003
  return h
}
function pad(n) {
  return n < 10 ? '0' + n : '' + n
}
function dateTime(ts) {
  const d = new Date(ts)
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes())
}

// ───────── 乘车人 ─────────
function passengers() {
  let list = wx.getStorageSync('tr_passengers')
  if (!list || !list.length) {
    list = SEED_PASSENGERS.slice()
    wx.setStorageSync('tr_passengers', list)
  }
  return list
}
function findPassenger(id) {
  return passengers().find(p => p.id === id) || null
}
function addPassenger(p) {
  const list = passengers()
  const id = list.reduce((m, x) => Math.max(m, x.id), 0) + 1
  const item = Object.assign({ id, self: false }, p)
  list.push(item)
  wx.setStorageSync('tr_passengers', list)
  return item
}
function updatePassenger(id, patch) {
  wx.setStorageSync('tr_passengers', passengers().map(p => (p.id === id ? Object.assign({}, p, patch) : p)))
}
function removePassenger(id) {
  wx.setStorageSync('tr_passengers', passengers().filter(p => p.id !== id))
}
// 身份证中间打码：3301**********1236
function maskId(s) {
  return s.slice(0, 4) + '**********' + s.slice(-4)
}

// ───────── 票价 ─────────
// 票种：儿童票所有席别半价；学生票二等座 75%、普速硬座 / 硬卧半价，其余席别按全价（12306 规则）
function typeRate(type, seatName) {
  if (type === '儿童票') return 0.5
  if (type === '学生票') {
    if (seatName === '二等座') return 0.75
    if (seatName === '硬座' || seatName === '硬卧') return 0.5
  }
  return 1
}
function ticketPrice(seat, type) {
  return Math.round(seat.price * typeRate(type || '成人票', seat.name))
}

// ───────── 订单 ─────────
function orders() {
  const v = wx.getStorageSync('tr_orders')
  return Array.isArray(v) ? v : []
}
function saveOrders(list) {
  wx.setStorageSync('tr_orders', list)
}
function findOrder(id) {
  return orders().find(o => o.id === id) || null
}
function updateOrder(id, patch) {
  let hit = null
  const list = orders().map(o => {
    if (o.id !== id) return o
    hit = Object.assign({}, o, patch)
    return hit
  })
  saveOrders(list)
  return hit
}
function addOrder(o) {
  const list = orders()
  const order = Object.assign({ id: 'E' + Date.now() + Math.floor(Math.random() * 90 + 10), status: 'paid', created: Date.now() }, o)
  if (order.status === 'paid' && !order.tickets) order.tickets = assignTickets(order)
  list.unshift(order)
  saveOrders(list)
  return order
}
// 开车时刻（本地时区的时间戳）
function depAt(o) {
  const p = o.date.split('-').map(Number)
  const t = (o.depText || '00:00').split(':').map(Number)
  return new Date(p[0], p[1] - 1, p[2], t[0], t[1]).getTime()
}

// 出票：给每位乘车人分车厢与座位。高铁 / 动车按席别分车厢（商务座 01 车、一等座 02~03 车、二等座 04~08 车），
// 选了座的用选的字母，没选的按 A→F 补；普速卧铺给铺位，硬座给座号。同一单的人排在同一排 / 同一节车厢
function assignTickets(o) {
  const h = hash(o.id + o.no + o.date + o.seat)
  const names = o.names || []
  const types = o.types || []
  const chosen = (o.seats || []).slice()
  const k = /^K/.test(o.no)
  const letters = ['A', 'B', 'C', 'D', 'F']
  const left = letters.filter(l => chosen.indexOf(l) < 0)
  return names.map((name, i) => {
    const type = types[i] || '成人票'
    const price = typeof o.prices === 'object' && o.prices ? o.prices[i] : o.price
    let coach, seatNo
    if (!k) {
      coach = o.seat === '商务座' ? 1 : o.seat === '一等座' ? 2 + (h % 2) : 4 + (h % 5)
      const row = 1 + (h % (o.seat === '商务座' ? 3 : o.seat === '一等座' ? 13 : 17))
      seatNo = pad(row) + (chosen[i] || left.shift() || 'A') + '号'
    } else if (o.seat === '硬座') {
      coach = 3 + (h % 6)
      seatNo = '0' + pad((h + i) % 118 + 1) + '号'
    } else {
      const berths = o.seat === '软卧' ? ['下铺', '上铺'] : ['下铺', '中铺', '上铺']
      coach = o.seat === '软卧' ? 9 : 10 + (h % 3)
      seatNo = pad(1 + (h + i) % 11) + '号' + berths[(h + i) % berths.length]
    }
    return { name, type, price, coach: pad(coach), seatNo, text: pad(coach) + '车 ' + seatNo }
  })
}
// 票面：老订单（没出过 tickets）按姓名与座位偏好现算
function ticketList(o) {
  if (o.tickets && o.tickets.length) return o.tickets
  if (o.status === 'waiting') return (o.names || []).map((name, i) => ({ name, type: (o.types || [])[i] || '成人票', price: o.price, coach: '', seatNo: '', text: '待兑现' }))
  return assignTickets(o)
}
function gateOf(o) {
  return '检票口 ' + 'AB'.charAt(hash(o.no) % 2) + (hash(o.no + o.date) % 16 + 1)
}

// 退票手续费（12306）：开车前 8 天以上免费；48 小时以上不足 8 天 5%；24~48 小时 10%；不足 24 小时 20%。
// 按张计，尾数到 5 角，每张最低 2 元。候补订单还没出票：全额退
function roundHalf(x) {
  return Math.round(x * 2) / 2
}
function refundQuote(o, now) {
  if (o.status === 'waiting') return { rate: 0, fee: 0, amount: o.total, rule: '候补尚未兑现，预付款全额退回' }
  const h = (depAt(o) - (now || Date.now())) / HOUR
  let rate = 0.2
  let rule = '开车前不足 24 小时，按票价 20% 收取'
  if (h >= 192) {
    rate = 0
    rule = '开车前 8 天以上，免收退票费'
  } else if (h >= 48) {
    rate = 0.05
    rule = '开车前 48 小时~8 天，按票价 5% 收取'
  } else if (h >= 24) {
    rate = 0.1
    rule = '开车前 24~48 小时，按票价 10% 收取'
  }
  const fee = rate ? round2(ticketList(o).reduce((s, t) => s + Math.max(2, roundHalf(Number(t.price || o.price) * rate)), 0)) : 0
  return { rate, fee, amount: Math.max(0, round2(o.total - fee)), rule }
}
// 退票 / 取消候补：退款原路返回，记下手续费
function cancelOrder(id) {
  const o = findOrder(id)
  if (!o || o.status === 'cancelled') return null
  const q = refundQuote(o)
  return updateOrder(id, { status: 'cancelled', refund: { fee: q.fee, amount: q.amount, at: Date.now(), waiting: o.status === 'waiting' } })
}
// 改签：开车前、每单一次；同一线路换车次 / 日期 / 席别，票价高补差、低退差
function canChange(o) {
  return o.status === 'paid' && !o.changed && o.date >= D.today()
}
function changeOrder(id, n) {
  const o = findOrder(id)
  if (!o || !canChange(o)) return null
  const next = Object.assign({}, o, {
    no: n.no, date: n.date, depText: n.depText, arrText: n.arrText, seat: n.seat, price: n.price, prices: n.prices,
    total: n.total, seats: n.seats, changed: { no: o.no, date: o.date, depText: o.depText, seat: o.seat, total: o.total, diff: round2(n.total - o.total), at: Date.now() }
  })
  next.tickets = assignTickets(next)
  return updateOrder(id, next)
}
// 候补推进：到时间就兑现出票；发车日都过了还没兑现，自动取消并全额退款
function sync(now) {
  now = now || Date.now()
  let changed = 0
  const list = orders().map(o => {
    if (o.status !== 'waiting') return o
    if (o.date < D.today()) {
      changed++
      return Object.assign({}, o, { status: 'cancelled', refund: { fee: 0, amount: o.total, at: now, waiting: true } })
    }
    if (now - (o.created || 0) < FULFILL_MS) return o
    changed++
    const done = Object.assign({}, o, { status: 'paid', fulfilledAt: now })
    done.tickets = assignTickets(done)
    return done
  })
  if (changed) saveOrders(list)
  return changed
}
// 状态：已取消 / 发车日已过 → 已完成；其余待出行（候补的也算待出行）
function orderState(o) {
  if (o.status === 'cancelled') return { done: true, text: '已取消' }
  if (o.date < D.today()) return { done: true, text: '已出行' }
  if (o.status === 'waiting') return { done: false, text: '候补中' }
  return { done: false, text: '待出行' }
}
const DEFAULT_QUERY = { from: '北京', to: '上海', highSpeed: false }
function query() {
  return Object.assign({}, DEFAULT_QUERY, wx.getStorageSync('tr_query') || {})
}
function saveQuery(q) {
  wx.setStorageSync('tr_query', q)
}
module.exports = {
  FULFILL_MS, round2, dateTime,
  passengers, findPassenger, addPassenger, updatePassenger, removePassenger, maskId, typeRate, ticketPrice,
  orders, findOrder, updateOrder, addOrder, depAt, ticketList, gateOf, refundQuote, cancelOrder, canChange, changeOrder, sync, orderState,
  query, saveQuery
}
