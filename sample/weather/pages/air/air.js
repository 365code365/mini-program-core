const W = require('../../utils/data.js')

Page({
  data: { city: null, aqi: null, items: [], trend: [], advice: [], rank: 0 },
  onLoad(q) {
    const c = W.byId((q && q.id) || W.current())
    wx.setNavigationBarTitle({ title: c.name + ' · 空气质量' })
    const lv = W.aqiLevel(c.aqi)
    const items = W.pollutants(c).map(p => Object.assign(p, { pct: Math.min(100, Math.round(p.v / p.max * 100)), color: W.aqiLevel(Math.round(p.v / p.max * 200)).color }))
    // 未来 24 小时 AQI：围绕当前值小幅波动，夜间略高
    const now = new Date().getHours()
    const trend = []
    for (let i = 0; i < 24; i += 2) {
      const h = (now + i) % 24
      const v = Math.max(8, Math.round(c.aqi * (0.82 + 0.3 * Math.abs(Math.sin((h + c.seed) / 5)))))
      const l = W.aqiLevel(v)
      trend.push({ i, label: (h < 10 ? '0' : '') + h + '时', v, color: l.color })
    }
    let tmax = 1
    trend.forEach(t => { tmax = Math.max(tmax, t.v) })
    trend.forEach(t => { t.h = Math.round(30 + t.v / tmax * 120) })
    const sorted = W.CITIES.slice().sort((a, b) => a.aqi - b.aqi)
    const rank = sorted.findIndex(x => x.id === c.id) + 1
    const good = c.aqi <= 100
    const advice = [
      { t: '户外运动', d: good ? '适宜' : '减少', c: good ? '#3ccf7c' : '#f58d1f' },
      { t: '开窗通风', d: good ? '适宜' : '不宜', c: good ? '#3ccf7c' : '#f58d1f' },
      { t: '佩戴口罩', d: good ? '无需' : '建议', c: good ? '#3ccf7c' : '#f58d1f' },
      { t: '空气净化器', d: good ? '关闭' : '开启', c: good ? '#3ccf7c' : '#f58d1f' }
    ]
    this.setData({
      city: c, items, trend, advice, rank, total: W.CITIES.length,
      aqi: { v: c.aqi, name: lv.name, color: lv.color, tip: lv.tip, deg: Math.min(270, Math.round(c.aqi / 300 * 270)) }
    })
  }
})
