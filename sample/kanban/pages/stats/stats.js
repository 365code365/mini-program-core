const S = require('../../utils/store.js')
Page({
  data: { total: 0, cols: [], rate: 0, overdue: 0, dueToday: 0, lateList: [], prios: [], todayText: '' },
  onShow() {
    const list = S.load()
    const t = S.today()
    const total = list.length
    const max = Math.max(1, ...S.COLS.map(c => list.filter(x => x.col === c.key).length))
    const cols = S.COLS.map(c => {
      const n = list.filter(x => x.col === c.key).length
      return { key: c.key, name: c.name, color: c.color, count: n, width: Math.round(n * 100 / max) }
    })
    const done = cols[2].count
    // 逾期：没完成、截止日期早于今天（日期字符串 YYYY-MM-DD 可直接比大小）
    const late = list.filter(S.isOverdue)
    this.setData({
      total,
      cols,
      rate: total ? Math.round(done * 100 / total) : 0,
      overdue: late.length,
      dueToday: list.filter(x => x.col !== 'done' && x.due === t).length,
      lateList: late.map(c => ({ id: c.id, title: c.title, due: c.due, owner: S.member(c.owner).name })),
      prios: S.PRIOS.map((p, i) => ({ name: p.name, color: p.color, count: list.filter(x => x.prio === i && x.col !== 'done').length })),
      todayText: t
    })
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/card/card?id=' + e.currentTarget.dataset.id })
  }
})
