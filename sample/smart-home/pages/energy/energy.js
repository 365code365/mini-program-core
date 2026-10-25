const E = require('../../utils/energy.js')

Page({
  data: {
    period: 'week',
    periods: [{ key: 'week', name: '本周' }, { key: 'month', name: '本月' }],
    total: 0,
    cost: '0.00',
    bars: [],
    top: 5, // 默认只显示前 5，点「展开全部」看全部
    expanded: false
  },

  onShow() {
    this.load()
  },

  load() {
    const r = E.report(this.data.period)
    this.setData({ total: r.total, cost: r.cost, bars: r.bars })
  },

  onPeriod(e) {
    const p = e.currentTarget.dataset.p
    if (p === this.data.period) return
    this.setData({ period: p })
    this.load()
  },

  onExpand() {
    this.setData({ expanded: !this.data.expanded })
  }
})
