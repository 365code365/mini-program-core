// 模拟行情：不联网。每只股票有昨收价，tick() 让现价做一步随机游走（±10% 涨跌停）。
// 状态放在模块里：require 有缓存，所有页面看到的是同一份行情
const STOCKS = [
  ['600519', '贵州茅台', 1688.0],
  ['000858', '五粮液', 142.35],
  ['601318', '中国平安', 45.62],
  ['600036', '招商银行', 33.18],
  ['000001', '平安银行', 10.86],
  ['300750', '宁德时代', 198.4],
  ['002594', '比亚迪', 236.7],
  ['601012', '隆基绿能', 18.92],
  ['600900', '长江电力', 28.45],
  ['000333', '美的集团', 68.1],
  ['601888', '中国中免', 72.36],
  ['688981', '中芯国际', 52.8],
  ['600276', '恒瑞医药', 44.2],
  ['002415', '海康威视', 31.55]
].map(r => ({ code: r[0], name: r[1], prev: r[2] }))

function r2(x) {
  return Math.round(x * 100) / 100
}
// 每只股票自己的伪随机序列（线性同余），开盘走势可复现
function rng(seed) {
  let s = seed >>> 0
  return function () {
    s = (s * 1664525 + 1013904223) >>> 0
    return s / 4294967296
  }
}
function seedOf(code) {
  return code.split('').reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7)
}

const state = {}
let tickNo = 0
STOCKS.forEach(s => {
  const rand = rng(seedOf(s.code))
  // 开盘到现在的 48 个分时点（每 5 分钟一个）
  const pts = []
  let p = s.prev * (1 + (rand() - 0.5) * 0.02)
  for (let i = 0; i < 48; i++) {
    p = clamp(s.prev, p * (1 + (rand() - 0.5) * 0.01))
    pts.push(r2(p))
  }
  // 30 根日 K：从 30 天前倒推到昨收
  const k = []
  let c = s.prev * (1 + (rand() - 0.5) * 0.3)
  for (let i = 0; i < 30; i++) {
    const o = c
    c = i === 29 ? s.prev : c * (1 + (rand() - 0.5) * 0.06)
    const h = Math.max(o, c) * (1 + rand() * 0.015)
    const l = Math.min(o, c) * (1 - rand() * 0.015)
    k.push({ o: r2(o), c: r2(c), h: r2(h), l: r2(l) })
  }
  state[s.code] = { rand, pts, k, price: pts[pts.length - 1], open: pts[0], vol: Math.round(rand() * 50000 + 20000) }
})

function clamp(prev, p) {
  return Math.min(prev * 1.1, Math.max(prev * 0.9, p))
}
function info(code) {
  return STOCKS.find(s => s.code === code) || null
}

// 走一步：每只股票随机涨跌 ±0.4% 以内，至少动一分钱
function tick() {
  tickNo++
  STOCKS.forEach(s => {
    const st = state[s.code]
    const step = (st.rand() - 0.5) * 0.008 * st.price
    const d = Math.abs(step) < 0.01 ? (step < 0 ? -0.01 : 0.01) : step
    st.price = r2(clamp(s.prev, st.price + d))
    st.pts[st.pts.length - 1] = st.price
    st.vol += Math.round(st.rand() * 800)
  })
  return tickNo
}
function ticks() {
  return tickNo
}

function quote(code) {
  const s = info(code)
  if (!s) return null
  const st = state[code]
  const chg = r2(st.price - s.prev)
  const pct = r2((chg / s.prev) * 100)
  const high = Math.max.apply(null, st.pts)
  const low = Math.min.apply(null, st.pts)
  return {
    code, name: s.name, price: st.price, prev: s.prev, open: st.open, high: r2(high), low: r2(low), vol: st.vol,
    chg, pct, dir: chg > 0 ? 'up' : chg < 0 ? 'down' : 'flat',
    priceText: st.price.toFixed(2),
    chgText: (chg > 0 ? '+' : '') + chg.toFixed(2),
    pctText: (pct > 0 ? '+' : '') + pct.toFixed(2) + '%'
  }
}

// 分时：柱高按当日最高最低归一化（0~100）
function intraday(code) {
  const st = state[code]
  const prev = info(code).prev
  const hi = Math.max(prev * 1.005, Math.max.apply(null, st.pts))
  const lo = Math.min(prev * 0.995, Math.min.apply(null, st.pts))
  return st.pts.map((p, i) => ({ i, h: Math.round(((p - lo) / (hi - lo)) * 90) + 6, dir: p >= prev ? 'up' : 'down' }))
}
// 日 K：实体与影线的上下位置（百分比）
function kline(code) {
  const k = state[code].k
  const hi = Math.max.apply(null, k.map(x => x.h))
  const lo = Math.min.apply(null, k.map(x => x.l))
  const y = v => Math.round(((hi - v) / (hi - lo)) * 1000) / 10
  return k.map((x, i) => ({
    i, dir: x.c >= x.o ? 'up' : 'down',
    wickTop: y(x.h), wickH: Math.max(0.5, y(x.l) - y(x.h)),
    bodyTop: y(Math.max(x.o, x.c)), bodyH: Math.max(0.8, y(Math.min(x.o, x.c)) - y(Math.max(x.o, x.c)))
  }))
}
// 五档盘口：卖五 → 卖一，买一 → 买五
function book(code) {
  const st = state[code]
  const rand = rng(seedOf(code) + tickNo)
  const tickSize = 0.01
  const row = (label, p) => ({ label, price: r2(p).toFixed(2), vol: Math.round(rand() * 900 + 20) })
  const sells = []
  for (let i = 5; i >= 1; i--) sells.push(row('卖' + i, st.price + tickSize * i))
  const buys = []
  for (let i = 1; i <= 5; i++) buys.push(row('买' + i, st.price - tickSize * (i - 1)))
  return { sells, buys }
}

module.exports = { STOCKS, r2, info, tick, ticks, quote, intraday, kline, book }
