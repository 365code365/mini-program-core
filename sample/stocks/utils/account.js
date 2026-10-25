// 自选、资金、持仓、成交记录：落在本地 storage
const M = require('./market.js')
const K_WATCH = 'stk_watch'
const K_CASH = 'stk_cash'
const K_POS = 'stk_pos'
const K_ORDERS = 'stk_orders'
const INIT_CASH = 100000

function watchlist() {
  const w = wx.getStorageSync(K_WATCH)
  if (Array.isArray(w)) return w
  const seed = ['600519', '601318', '300750', '000001', '600036', '002594']
  wx.setStorageSync(K_WATCH, seed)
  return seed
}
function isWatched(code) {
  return watchlist().indexOf(code) >= 0
}
function addWatch(code) {
  const w = watchlist()
  if (w.indexOf(code) >= 0) return false
  w.push(code)
  wx.setStorageSync(K_WATCH, w)
  return true
}
function removeWatch(code) {
  wx.setStorageSync(K_WATCH, watchlist().filter(c => c !== code))
}

function cash() {
  const c = wx.getStorageSync(K_CASH)
  if (typeof c === 'number') return c
  wx.setStorageSync(K_CASH, INIT_CASH)
  return INIT_CASH
}
// 持仓：{ code: { qty, cost } }，cost 是持仓均价
function positions() {
  const p = wx.getStorageSync(K_POS)
  if (p && typeof p === 'object' && !Array.isArray(p)) return p
  const seed = { '601318': { qty: 500, cost: 44.1 }, '000001': { qty: 1000, cost: 11.2 } }
  wx.setStorageSync(K_POS, seed)
  return seed
}
function holding(code) {
  const p = positions()[code]
  return p ? p.qty : 0
}
function orders() {
  return wx.getStorageSync(K_ORDERS) || []
}

// 下单（已校验过）：按 price 全部成交。返回成交记录
function execute(code, side, price, qty) {
  const pos = positions()
  let c = cash()
  const amount = M.r2(price * qty)
  const p = pos[code] || { qty: 0, cost: 0 }
  if (side === 'buy') {
    p.cost = M.r2((p.cost * p.qty + amount) / (p.qty + qty) * 1000) / 1000
    p.qty += qty
    c = M.r2(c - amount)
    pos[code] = p
  } else {
    p.qty -= qty
    c = M.r2(c + amount)
    if (p.qty <= 0) delete pos[code]
    else pos[code] = p
  }
  wx.setStorageSync(K_POS, pos)
  wx.setStorageSync(K_CASH, c)
  const rec = { id: orders().length + 1, code, name: M.info(code).name, side, price, qty, amount, at: Date.now() }
  wx.setStorageSync(K_ORDERS, [rec].concat(orders()))
  return rec
}

// 持仓汇总：市值、浮动盈亏按当前行情算
function summary() {
  const pos = positions()
  const rows = Object.keys(pos).map(code => {
    const q = M.quote(code)
    const p = pos[code]
    const mv = M.r2(q.price * p.qty)
    const pnl = M.r2((q.price - p.cost) * p.qty)
    return {
      code, name: q.name, qty: p.qty, cost: p.cost, price: q.price, priceText: q.priceText, mv, pnl,
      pnlPct: M.r2((pnl / (p.cost * p.qty)) * 100), dir: pnl > 0 ? 'up' : pnl < 0 ? 'down' : 'flat'
    }
  })
  const mv = M.r2(rows.reduce((s, r) => s + r.mv, 0))
  const pnl = M.r2(rows.reduce((s, r) => s + r.pnl, 0))
  const c = cash()
  return { rows, mv, pnl, cash: c, total: M.r2(mv + c), dir: pnl > 0 ? 'up' : pnl < 0 ? 'down' : 'flat' }
}

module.exports = { INIT_CASH, watchlist, isWatched, addWatch, removeWatch, cash, positions, holding, orders, execute, summary }
