// 本地状态：我的快递 / 查询历史 / 地址簿 / 寄件订单 / 寄件券 / 设置，全部存在 storage 里，各页读同一份
const D = require('./data.js')
const K = {
  list: 'express_list', history: 'express_history', addr: 'express_addr', orders: 'express_orders',
  coupons: 'express_coupons', subs: 'express_subs', settings: 'express_settings', hidden: 'express_hidden'
}
const MIN = 60000
const HOUR = 60 * MIN
const CANCEL_MIN = 30   // 下单 30 分钟内（快递员上门前）可以取消
function round2(n) { return Math.round(n * 100) / 100 }
function pad(n) { return (n < 10 ? '0' : '') + n }
function stamp(ts) {
  const d = new Date(ts || Date.now())
  return pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes())
}
// getStorageSync 缺省给 ''：数组按类型判。删光了就是空数组，不再回填示例数据
function arr(key, seed) {
  const v = wx.getStorageSync(key)
  if (Array.isArray(v)) return v
  const s = seed ? JSON.parse(JSON.stringify(seed)) : []
  if (seed) wx.setStorageSync(key, s)
  return s
}
function maskPhone(p) {
  return /^1\d{10}$/.test(p) ? p.slice(0, 3) + '****' + p.slice(7) : p
}
// ───────── 地址簿 ─────────
function getAddrs() { return arr(K.addr, D.ADDRESSES) }
function findAddr(id) { return getAddrs().find(a => String(a.id) === String(id)) || null }
function defaultAddr() { const l = getAddrs(); return l.find(a => a.isDefault) || l[0] || null }
function saveAddr(a) {
  let l = getAddrs()
  if (!a.id) a.id = Date.now()
  if (a.isDefault || !l.some(x => x.id !== a.id)) {
    l = l.map(x => Object.assign({}, x, { isDefault: false }))
    a.isDefault = true
  }
  const i = l.findIndex(x => x.id === a.id)
  if (i >= 0) l[i] = a
  else l.push(a)
  wx.setStorageSync(K.addr, l)
  return a
}
function setDefaultAddr(id) {
  wx.setStorageSync(K.addr, getAddrs().map(x => Object.assign({}, x, { isDefault: String(x.id) === String(id) })))
}
function removeAddr(id) {
  const l = getAddrs().filter(x => String(x.id) !== String(id))
  if (l.length && !l.some(x => x.isDefault)) l[0].isDefault = true
  wx.setStorageSync(K.addr, l)
}
// 「广东省 深圳市 南山区」→ 深圳；直辖市取第一段
function cityOf(a) {
  const p = String(a.region || '').split(' ')
  return (p[1] && p[1] !== p[0] ? p[1] : p[0] || '').replace(/[省市]$/, '')
}
// ───────── 寄件券 ─────────
function getCoupons() {
  return arr(K.coupons, [
    { id: 'q1', title: '寄件立减券', amount: 5, min: 0, expire: '2026-11-30', status: 'valid' },
    { id: 'q2', title: '满 20 减 3', amount: 3, min: 20, expire: '2026-12-31', status: 'valid' },
    { id: 'q0', title: '新人体验券', amount: 2, min: 0, expire: '2026-10-31', status: 'used' }
  ])
}
function usableCoupons(fee) {
  return getCoupons().filter(c => c.status === 'valid' && fee >= c.min)
}
function bestCoupon(fee) {
  return usableCoupons(fee).sort((a, b) => b.amount - a.amount)[0] || null
}
function setCoupon(id, status) {
  wx.setStorageSync(K.coupons, getCoupons().map(c => (c.id === id ? Object.assign({}, c, { status }) : c)))
}
// ───────── 设置 ─────────
function settings() {
  const v = wx.getStorageSync(K.settings)
  return Object.assign({ push: true, privacy: true }, v && typeof v === 'object' ? v : {})
}
function setSetting(k, on) {
  const s = settings()
  s[k] = !!on
  wx.setStorageSync(K.settings, s)
  return s
}
// ───────── 寄件订单 ─────────
function getOrders() { return arr(K.orders) }
function saveOrders(l) { wx.setStorageSync(K.orders, l) }
function findOrder(id) { return getOrders().find(o => o.id === String(id)) || null }
// 进度按下单时间推进：30 分钟内待揽收（可取消）→ 运输中 → 第 2 天派送中 → 已签收
const STAGES = [
  { key: 'pending', text: '待揽收', at: 0 },
  { key: 'transit', text: '运输中', at: CANCEL_MIN * MIN },
  { key: 'deliver', text: '派送中', at: 24 * HOUR },
  { key: 'signed', text: '已签收', at: 30 * HOUR }
]
function stageOf(o, now) {
  if (o.status === 'cancelled') return { key: 'cancelled', text: '已取消', idx: -1, canCancel: false }
  const t = (now || Date.now()) - o.createdAt
  let idx = 0
  STAGES.forEach((s, i) => { if (t >= s.at) idx = i })
  return { key: STAGES[idx].key, text: STAGES[idx].text, idx, canCancel: idx === 0 }
}
// 寄件单在「我的快递」里的那一条：轨迹按阶段生成
function pkgOf(o) {
  const st = stageOf(o)
  const c = D.CARRIERS[o.carrier]
  const at = i => stamp(o.createdAt + STAGES[i].at)
  const traces = [{ time: stamp(o.createdAt), text: '订单已提交，快递员 ' + o.courier.name + ' 将于 ' + o.timeText + ' 上门取件' }]
  if (st.idx >= 1) traces.unshift({ time: at(1), text: c.name + ' 已揽收，快件发往【' + o.toCity + '转运中心】' })
  if (st.idx >= 2) traces.unshift({ time: at(2), text: '【' + o.toCity + '】快递员正在派件，请保持电话畅通' })
  if (st.idx >= 3) traces.unshift({ time: at(3), text: '已签收，签收人 ' + o.receiver.name + '，感谢使用' + c.name })
  return {
    no: o.no, carrier: o.carrier, status: st.key, remark: '寄件：' + o.goods, from: o.fromCity, to: o.toCity, orderId: o.id,
    eta: ['等待快递员上门揽收', '预计 ' + o.eta, '今天送达', stamp(o.createdAt + STAGES[3].at) + ' 已签收'][st.idx],
    courier: o.courier, traces
  }
}
function createOrder(info) {
  const p = D.PRODUCTS.find(x => x.key === info.product)
  const n = Date.now()
  const no = (p.key === 'sf' ? 'SF' : p.key === 'jd' ? 'JD' : '78') + String(n).slice(-12)
  const o = Object.assign({
    id: 'JJ' + String(n).slice(-10), no, carrier: p.key, productName: p.name, eta: p.eta, createdAt: n, status: 'ok',
    fromCity: cityOf(info.sender), toCity: cityOf(info.receiver),
    courier: { name: '刘师傅', phone: '137 5810 3326', rate: '4.9', count: '9800' }
  }, info)
  const l = getOrders()
  l.unshift(o)
  saveOrders(l)
  if (o.couponId) setCoupon(o.couponId, 'used')
  return o
}
// 取消：运费原路退回、券退回，「我的快递」里那条一并去掉
function cancelOrder(id) {
  const l = getOrders()
  const o = l.find(x => x.id === id)
  if (!o || !stageOf(o).canCancel) return null
  o.status = 'cancelled'
  o.cancelledAt = Date.now()
  saveOrders(l)
  if (o.couponId) setCoupon(o.couponId, 'valid')
  return o
}
// ───────── 我的快递 / 查询历史 ─────────
// 读列表时把寄件单同步进去：没取消的按当前阶段刷新，取消了的去掉
function getList() {
  const orders = getOrders()
  const mine = {}
  orders.forEach(o => { mine[o.no] = o })
  let l = arr(K.list, D.SEED).filter(i => !mine[i.no] || mine[i.no].status !== 'cancelled')
  const hidden = arr(K.hidden)
  orders.filter(o => o.status !== 'cancelled' && hidden.indexOf(o.no) < 0).forEach(o => {
    const pkg = pkgOf(o)
    const i = l.findIndex(x => x.no === o.no)
    if (i >= 0) l[i] = Object.assign({}, l[i], pkg, { remark: l[i].remark || pkg.remark })
    else l.unshift(pkg)
  })
  return l
}
function saveList(l) { wx.setStorageSync(K.list, l) }
function findPkg(no) { return getList().find(i => i.no === no) || null }
// 删掉的寄件单记进 hidden：getList 不再把它同步回来（订单本身还在寄件记录里）
function removePkg(no) {
  saveList(getList().filter(i => i.no !== no))
  if (getOrders().some(o => o.no === no)) wx.setStorageSync(K.hidden, arr(K.hidden).concat([no]))
}
// 查一个单号：没有就生成一条轨迹放进「我的快递」，记进查询历史
function query(no) {
  wx.setStorageSync(K.hidden, arr(K.hidden).filter(x => x !== no))
  const l = getList()
  if (!l.some(i => i.no === no)) {
    l.unshift(D.mockTrack(no, D.detectCarrier(no)))
    saveList(l)
  }
  wx.setStorageSync(K.history, [no].concat(history().filter(x => x !== no)).slice(0, 8))
}
function history() { return arr(K.history) }
function clearHistory() { wx.setStorageSync(K.history, []) }
function subs() { return arr(K.subs) }
function toggleSub(no) {
  const l = subs()
  const on = l.indexOf(no) < 0
  wx.setStorageSync(K.subs, on ? l.concat([no]) : l.filter(x => x !== no))
  return on
}
module.exports = {
  MIN, CANCEL_MIN, round2, stamp, maskPhone,
  getAddrs, findAddr, defaultAddr, saveAddr, setDefaultAddr, removeAddr, cityOf,
  getCoupons, usableCoupons, bestCoupon, settings, setSetting,
  getOrders, findOrder, stageOf, createOrder, cancelOrder,
  getList, saveList, findPkg, removePkg, query, history, clearHistory, subs, toggleSub
}
