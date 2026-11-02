// 本地 storage：我的车辆 pk_cars、在场记录 pk_sessions、订单 pk_orders、各停车场空位 pk_free、
// 月卡 pk_cards、优惠券用量 pk_coupon_used、发票 pk_invoices（已开票的单号记在 pk_invoiced）
const { LOTS, COUPONS, PLANS, lotById } = require('./data.js')

function get(key, seed) {
  const v = wx.getStorageSync(key)
  if (v === '' || v === undefined || v === null) {
    const s = typeof seed === 'function' ? seed() : seed
    wx.setStorageSync(key, s)
    return s
  }
  return v
}
function set(key, v) {
  wx.setStorageSync(key, v)
}

function cars() {
  return get('pk_cars', [
    { plate: '京A12345', newEnergy: false },
    { plate: '沪AD12345', newEnergy: true }
  ])
}
function saveCars(list) { set('pk_cars', list) }

// 在场记录：车辆已经进了哪个停车场、几点入场（模拟道闸推送）
function sessions() {
  return get('pk_sessions', [
    { plate: '京A12345', lotId: 1, inTime: '15:10' },
    { plate: '沪AD12345', lotId: 3, inTime: '09:40' }
  ])
}
function saveSessions(list) { set('pk_sessions', list) }
function sessionOf(plate) {
  return sessions().find(s => s.plate === plate) || null
}

function orders() {
  return get('pk_orders', [
    { id: 'P1000', type: 'pay', status: 'done', statusText: '已支付', plate: '京A12345', lotId: 3, lotName: '市民中心 P3 停车楼', desc: '08:20 入场 · 停车 2小时45分钟', amount: 14 }
  ])
}
function saveOrders(list) { set('pk_orders', list) }
// 订单号：类型前缀 + 递增序号（演示环境可确定）
function nextOrderId(prefix) {
  return prefix + (1001 + orders().length)
}

function freeMap() {
  const m = {}
  LOTS.forEach(l => { m[l.id] = l.free })
  return get('pk_free', m)
}
function freeOf(id) {
  return Number(freeMap()[id])
}
function addFree(id, n) {
  const m = freeMap()
  const lot = lotById(id)
  m[id] = Math.max(0, Math.min(lot.total, Number(m[id]) + n))
  set('pk_free', m)
}

function round2(n) { return Math.round(n * 100) / 100 }
function pad(n) { return (n < 10 ? '0' : '') + n }
function dateStr(ts) {
  const d = new Date(ts)
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate())
}
function today() { return dateStr(Date.now()) }
function stamp() {
  const d = new Date()
  return dateStr(d.getTime()).slice(5) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes())
}
// 只读不写：新加的几项缺省时不往 storage 里种数据，免得改变老数据的样子
function peek(key, def) {
  const v = wx.getStorageSync(key)
  return v === '' || v === undefined || v === null ? def : v
}
// ───────── 优惠券：每种有固定张数，付款用掉一张 ─────────
function couponLeft(idx) {
  const c = COUPONS[idx]
  if (!c || !c.stock) return Infinity
  const used = peek('pk_coupon_used', {})
  return Math.max(0, c.stock - Number(used[idx] || 0))
}
function useCoupon(idx) {
  if (!COUPONS[idx] || !COUPONS[idx].stock) return
  const used = peek('pk_coupon_used', {})
  used[idx] = Number(used[idx] || 0) + 1
  set('pk_coupon_used', used)
}
// ───────── 月卡 ─────────
function cards() { return peek('pk_cards', []) }
function cardFor(plate, lotId) {
  const t = today()
  return cards().find(c => c.plate === plate && c.lotId === Number(lotId) && c.end >= t) || null
}
function planPrice(lot, plan) { return round2(lot.month * plan.rate) }
// 办卡 / 续费：同车同场有未过期的卡就顺延，否则从今天起算
function buyCard(plate, lotId, planKey) {
  const lot = lotById(lotId)
  const plan = PLANS.find(p => p.key === planKey)
  const list = cards()
  const hit = list.find(c => c.plate === plate && c.lotId === lot.id && c.end >= today())
  const from = hit ? new Date(hit.end.replace(/-/g, '/')).getTime() + 86400000 : Date.now()
  const end = dateStr(from + (plan.days - 1) * 86400000)
  const price = planPrice(lot, plan)
  const rec = { id: 'M' + Date.now().toString().slice(-8), plate, lotId: lot.id, lotName: lot.name, plan: plan.name, price, time: stamp() }
  if (hit) {
    hit.end = end
    hit.logs = (hit.logs || []).concat([rec])
  } else {
    list.unshift({ id: rec.id, plate, lotId: lot.id, lotName: lot.name, start: today(), end, logs: [rec] })
  }
  set('pk_cards', list)
  return { card: hit || list[0], rec }
}
// ───────── 发票：停车缴费（实付 > 0）与月卡购买记录都能开 ─────────
function invoiced() { return peek('pk_invoiced', []) }
function invoiceable() {
  const done = invoiced()
  const pays = orders().filter(o => o.type === 'pay' && o.status === 'done' && o.amount > 0)
    .map(o => ({ id: o.id, title: o.lotName, desc: o.plate + ' · 停车缴费', amount: o.amount }))
  const buys = []
  cards().forEach(c => (c.logs || []).forEach(r => buys.push({ id: r.id, title: r.lotName, desc: r.plate + ' · ' + r.plan, amount: r.price })))
  return buys.concat(pays).filter(x => done.indexOf(x.id) < 0)
}
function invoices() { return peek('pk_invoices', []) }
function addInvoice(ids, info) {
  const can = invoiceable().filter(x => ids.indexOf(x.id) >= 0)
  if (!can.length) return null
  const inv = Object.assign({ id: 'FP' + Date.now().toString().slice(-8), amount: round2(can.reduce((s, x) => s + x.amount, 0)), ids: can.map(x => x.id), time: stamp() }, info)
  set('pk_invoices', [inv].concat(invoices()))
  set('pk_invoiced', invoiced().concat(inv.ids))
  return inv
}
module.exports = {
  cars, saveCars, sessions, saveSessions, sessionOf, orders, saveOrders, nextOrderId, freeMap, freeOf, addFree,
  round2, today, stamp, couponLeft, useCoupon, cards, cardFor, planPrice, buyCard, invoiced, invoiceable, invoices, addInvoice
}
