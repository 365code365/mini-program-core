const S = require('../../utils/store.js')

Page({
  data: {
    games: 0,
    avg: 0,
    top: 0,
    maxTile: 0,
    bests: [], // 各尺寸最高分
    tiles: [] // 最大方块分布：[{ v, n, pct }]
  },

  onShow() {
    const list = S.loadHistory()
    const total = list.reduce((m, r) => m + r.score, 0)
    const dist = {}
    list.forEach(r => { dist[r.maxTile] = (dist[r.maxTile] || 0) + 1 })
    const keys = Object.keys(dist).map(Number).sort((a, b) => b - a)
    const most = keys.reduce((m, k) => Math.max(m, dist[k]), 1)
    this.setData({
      games: list.length,
      avg: list.length ? Math.round(total / list.length) : 0,
      top: list.reduce((m, r) => Math.max(m, r.score), 0),
      maxTile: list.reduce((m, r) => Math.max(m, r.maxTile), 0),
      bests: S.SIZES.map(n => ({ n, best: S.loadBest(n) })),
      tiles: keys.map(v => ({ v, n: dist[v], pct: Math.round(dist[v] / most * 100) }))
    })
  }
})
