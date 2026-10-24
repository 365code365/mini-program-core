const store = require('../../utils/store.js')

const PRESETS = [3000, 5000, 8000, 10000, 15000]

Page({
  data: { budget: 8000, spent: 0, spentText: '0.00', pct: 0, leftText: '0.00', over: false, perDayText: '0.00', daysLeft: 1, presets: PRESETS, remind: true },
  onLoad() {
    const now = new Date()
    const list = store.monthRecords(now.getFullYear(), now.getMonth() + 1)
    const dim = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate()
    this.spent = store.sum(list, 'out')
    this.setData({ spentText: store.money(this.spent), daysLeft: dim - now.getDate() + 1, remind: wx.getStorageSync('ledger_remind') !== false })
    this.apply(store.getBudget())
  },
  apply(b) {
    const left = b - this.spent
    this.setData({
      budget: b,
      pct: Math.min(100, Math.round(this.spent / b * 100)),
      over: left < 0,
      leftText: store.money(Math.abs(left)),
      perDayText: store.money(Math.max(0, left) / this.data.daysLeft)
    })
  },
  onSliding(e) { this.apply(e.detail.value) },
  onSlide(e) { this.apply(e.detail.value) },
  onPreset(e) { this.apply(+e.currentTarget.dataset.v) },
  onRemind(e) { this.setData({ remind: e.detail.value }) },
  onSave() {
    store.setBudget(this.data.budget)
    wx.setStorageSync('ledger_remind', this.data.remind)
    wx.showToast({ title: '预算已保存', icon: 'success' })
    setTimeout(() => wx.navigateBack(), 600)
  }
})
