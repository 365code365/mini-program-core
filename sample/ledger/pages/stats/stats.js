const store = require('../../utils/store.js')

Page({
  data: {
    y: 0,
    m: 0,
    monthText: '',
    isCurrent: true,
    type: 'out',
    totalText: '0.00',
    otherText: '0.00',
    count: 0,
    avgText: '0.00',
    cats: [],
    days: [],
    top: [],
    empty: false
  },
  onLoad() {
    const now = new Date()
    this.setData({ y: now.getFullYear(), m: now.getMonth() + 1 })
  },
  onShow() { this.refresh() },
  onPrev() {
    let { y, m } = this.data
    m -= 1; if (m < 1) { m = 12; y -= 1 }
    this.setData({ y, m }); this.refresh()
  },
  onNext() {
    if (this.data.isCurrent) { wx.showToast({ title: '已经是本月了', icon: 'none' }); return }
    let { y, m } = this.data
    m += 1; if (m > 12) { m = 1; y += 1 }
    this.setData({ y, m }); this.refresh()
  },
  onType(e) { this.setData({ type: e.currentTarget.dataset.t }); this.refresh() },
  onItem(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }) },
  refresh() {
    const { y, m, type } = this.data
    const now = new Date()
    const isCurrent = y === now.getFullYear() && m === now.getMonth() + 1
    const list = store.monthRecords(y, m)
    const mine = list.filter(r => r.type === type)
    const total = store.sum(list, type)
    const other = store.sum(list, type === 'out' ? 'in' : 'out')
    // 分类汇总
    const map = {}
    mine.forEach(r => { map[r.cat] = (map[r.cat] || 0) + r.amount })
    const cats = Object.keys(map).map(name => {
      const c = store.catOf(type, name)
      return { name, ch: c.ch, color: c.color, v: map[name] }
    }).sort((a, b) => b.v - a.v)
    const max = cats.length ? cats[0].v : 1
    cats.forEach(c => {
      c.pct = total ? Math.round(c.v / total * 1000) / 10 : 0
      c.w = Math.max(3, Math.round(c.v / max * 100))
      c.vText = store.money(c.v)
    })
    // 每日柱状图
    const dim = new Date(y, m, 0).getDate()
    const lastDay = isCurrent ? now.getDate() : dim
    const daily = []
    for (let d = 1; d <= dim; d++) daily.push(0)
    mine.forEach(r => { daily[+r.date.slice(8) - 1] += r.amount })
    // 大额（如房租）会把其他柱子压扁：按 90 分位封顶
    const sorted = daily.slice().sort((a, b) => a - b)
    const cap = Math.max(1, sorted[Math.floor(sorted.length * 0.9)] * 1.3)
    const days = daily.map((v, i) => ({
      d: i + 1,
      h: v > 0 ? Math.max(6, Math.round(Math.min(v, cap) / cap * 100)) : 0,
      label: (i + 1) % 5 === 0 || i === 0 ? String(i + 1) : '',
      future: i + 1 > lastDay
    }))
    const top = mine.slice().sort((a, b) => b.amount - a.amount).slice(0, 6).map(r => {
      const d = store.decorate(r)
      d.dateText = (+r.date.slice(5, 7)) + '月' + (+r.date.slice(8)) + '日'
      return d
    })
    this.setData({
      isCurrent,
      monthText: y + '年' + m + '月',
      totalText: store.money(total),
      otherText: store.money(other),
      count: mine.length,
      avgText: store.money(total / lastDay),
      cats, days, top,
      empty: mine.length === 0
    })
  }
})
