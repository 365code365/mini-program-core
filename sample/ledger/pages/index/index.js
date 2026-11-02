const store = require('../../utils/store.js')

Page({
  data: {
    monthText: '',
    outText: '0.00',
    inText: '0.00',
    leftText: '0.00',
    budgetText: '0',
    budgetLeftText: '0.00',
    budgetPct: 0,
    budgetOver: false,
    perDayText: '0.00',
    groups: [],
    empty: false
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 })
    this.refresh()
  },
  refresh() {
    const now = new Date()
    const y = now.getFullYear()
    const m = now.getMonth() + 1
    const list = store.monthRecords(y, m)
    const out = store.sum(list, 'out')
    const inc = store.sum(list, 'in')
    const budget = store.getBudget()
    const daysInMonth = new Date(y, m, 0).getDate()
    const daysLeft = daysInMonth - now.getDate() + 1
    const left = budget - out
    // 按日分组
    const map = {}
    const order = []
    list.forEach(r => {
      if (!map[r.date]) { map[r.date] = { date: r.date, label: store.dayLabel(r.date), out: 0, inc: 0, items: [] }; order.push(r.date) }
      const g = map[r.date]
      g.items.push(store.decorate(r))
      if (r.type === 'out') g.out += r.amount; else g.inc += r.amount
    })
    const groups = order.map(d => {
      const g = map[d]
      return { date: g.date, label: g.label, items: g.items, outText: store.money(g.out), inText: store.money(g.inc), hasIn: g.inc > 0, hasOut: g.out > 0 }
    })
    this.setData({
      monthText: m + '月',
      bookName: store.curBook().name,
      outText: store.money(out),
      inText: store.money(inc),
      leftText: store.money(inc - out),
      leftYuan: store.yuan(inc - out),
      budgetText: String(budget),
      budgetLeftText: store.money(Math.abs(left)),
      budgetOver: left < 0,
      budgetWarn: left < 0 || (wx.getStorageSync('ledger_remind') !== false && out / budget >= 0.8),
      budgetPct: Math.min(100, Math.round(out / budget * 100)),
      perDayText: store.money(Math.max(0, left) / daysLeft),
      groups: groups,
      empty: groups.length === 0
    })
  },
  onAdd() { wx.navigateTo({ url: '/pages/add/add' }) },
  onBudget() { wx.navigateTo({ url: '/pages/budget/budget' }) },
  onBooks() { wx.navigateTo({ url: '/pages/books/books' }) },
  onStats() { wx.switchTab({ url: '/pages/stats/stats' }) },
  onItem(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }) }
})
