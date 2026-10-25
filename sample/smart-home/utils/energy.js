// 能耗数据（kWh）：按设备、按周 / 月，本地写死
const D = require('./devices.js')

const DATA = {
  week: { ac1: 18.6, ac2: 12.4, light1: 3.2, light2: 0.9, light3: 1.4, light4: 0.8, tv1: 4.5, fridge1: 7.0, hood1: 1.1, humid1: 2.3, curtain1: 0.1, speaker1: 0.3 },
  month: { ac1: 80.2, ac2: 55.0, light1: 13.6, light2: 4.1, light3: 6.2, light4: 3.5, tv1: 19.8, fridge1: 30.4, hood1: 4.9, humid1: 9.7, curtain1: 0.4, speaker1: 1.2 }
}

// 每度电 0.56 元
const PRICE = 0.56

// 返回 { total, cost, bars: [{ id, name, kwh, pct }] }，按耗电从高到低；条形宽度 = 本设备 / 最高 × 100%
function report(period) {
  const table = DATA[period] || DATA.week
  const devices = D.load()
  const rows = devices.map(d => ({ id: d.id, name: d.name, room: D.roomName(d.room), kwh: table[d.id] || 0 }))
  rows.sort((a, b) => b.kwh - a.kwh)
  const max = rows.length ? rows[0].kwh : 1
  const total = rows.reduce((m, r) => m + r.kwh, 0)
  return {
    total: Math.round(total * 10) / 10,
    cost: (total * PRICE).toFixed(2),
    bars: rows.map(r => Object.assign(r, { pct: Math.round(r.kwh / max * 100) }))
  }
}

module.exports = { report, PRICE }
