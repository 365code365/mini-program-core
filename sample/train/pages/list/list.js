const D = require('../../utils/data.js')
Page({
  data: {
    from: '',
    to: '',
    date: '',
    dateText: '',
    canPrev: false,
    highSpeed: false,
    periods: D.PERIODS.map(p => Object.assign({ on: false }, p)),
    sorts: D.SORTS,
    sort: 0,
    trains: [],
    total: 0
  },
  onLoad(q) {
    const date = q.date || D.today()
    this.setData({ from: q.from || '北京', to: q.to || '上海', highSpeed: q.gd === '1' })
    wx.setNavigationBarTitle({ title: this.data.from + ' → ' + this.data.to })
    this.setDate(date)
  },
  setDate(date) {
    this.all = D.trains(this.data.from, this.data.to, date)
    this.setData({ date, dateText: D.dateLabel(date), canPrev: date > D.today() })
    this.refresh()
  },
  // 筛选（车型 + 出发时段多选，时段一个都不选 = 不限）→ 排序
  refresh() {
    const on = this.data.periods.filter(p => p.on)
    const key = D.SORTS[this.data.sort].key
    const list = this.all
      .filter(t => !this.data.highSpeed || D.isHighSpeed(t))
      .filter(t => !on.length || on.some(p => t.dep >= p.from && t.dep < p.to))
      .slice()
      .sort((a, b) => (key === 'dep' ? a.dep - b.dep : key === 'dur' ? a.dur - b.dur || a.dep - b.dep : a.minPrice - b.minPrice || a.dep - b.dep))
    this.setData({ trains: list, total: list.length })
  },
  onPrevDay() {
    if (this.data.canPrev) this.setDate(D.addDays(this.data.date, -1))
  },
  onNextDay() {
    this.setDate(D.addDays(this.data.date, 1))
  },
  onHigh() {
    this.setData({ highSpeed: !this.data.highSpeed })
    this.refresh()
  },
  onPeriod(e) {
    const i = Number(e.currentTarget.dataset.index)
    this.setData({ ['periods[' + i + '].on']: !this.data.periods[i].on })
    this.refresh()
  },
  onResetPeriod() {
    this.setData({ periods: this.data.periods.map(p => Object.assign({}, p, { on: false })) })
    this.refresh()
  },
  onSort(e) {
    this.setData({ sort: Number(e.currentTarget.dataset.index) })
    this.refresh()
  },
  onTrain(e) {
    const t = this.data.trains[Number(e.currentTarget.dataset.index)]
    if (t.seats.every(s => s.left === 0)) {
      wx.showToast({ title: t.no + ' 已售罄', icon: 'none' })
      return
    }
    getApp().globalData.train = t
    wx.navigateTo({ url: '/pages/book/book?no=' + t.no })
  }
})
