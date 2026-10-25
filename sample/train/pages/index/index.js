const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
Page({
  data: {
    from: '北京',
    to: '上海',
    date: '',
    dateText: '',
    start: '',
    end: '',
    highSpeed: false,
    history: [],
    swapping: false
  },
  onLoad() {
    const t = D.today()
    const q = S.query()
    this.setData({ from: q.from, to: q.to, highSpeed: q.highSpeed, date: t, dateText: D.dateLabel(t), start: t, end: D.addDays(t, 29) })
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 0 })
    // 城市选择页选完回来：取走结果
    const g = getApp().globalData
    if (g.pickedCity) {
      const p = g.pickedCity
      g.pickedCity = null
      const patch = { [p.field]: p.name }
      // 选成和另一端同一个城市：两端互换，避免「北京 → 北京」
      const other = p.field === 'from' ? 'to' : 'from'
      if (this.data[other] === p.name) patch[other] = this.data[p.field]
      this.setData(patch)
      this.persist()
    }
    this.setData({ history: wx.getStorageSync('tr_history') || [] })
  },
  persist() {
    S.saveQuery({ from: this.data.from, to: this.data.to, highSpeed: this.data.highSpeed })
  },
  onCity(e) {
    const field = e.currentTarget.dataset.field
    wx.navigateTo({ url: '/pages/city/city?field=' + field + '&current=' + this.data[field] })
  },
  onSwap() {
    this.setData({ from: this.data.to, to: this.data.from, swapping: !this.data.swapping })
    this.persist()
  },
  // date 模式：e.detail.value 是 'YYYY-MM-DD' 字符串
  onDate(e) {
    const date = e.detail.value
    this.setData({ date, dateText: D.dateLabel(date) })
  },
  onHigh(e) {
    this.setData({ highSpeed: e.detail.value })
    this.persist()
  },
  onSearch() {
    const { from, to, date, highSpeed } = this.data
    if (from === to) {
      wx.showToast({ title: '出发地和目的地不能相同', icon: 'none' })
      return
    }
    const key = from + '-' + to
    const history = [key].concat(this.data.history.filter(h => h !== key)).slice(0, 4)
    wx.setStorageSync('tr_history', history)
    this.setData({ history })
    wx.navigateTo({ url: '/pages/list/list?from=' + from + '&to=' + to + '&date=' + date + '&gd=' + (highSpeed ? 1 : 0) })
  },
  onHistory(e) {
    const p = this.data.history[Number(e.currentTarget.dataset.index)].split('-')
    this.setData({ from: p[0], to: p[1] })
    this.persist()
  },
  onClearHistory() {
    wx.removeStorageSync('tr_history')
    this.setData({ history: [] })
  }
})
