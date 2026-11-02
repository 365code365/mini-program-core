const store = require('../../utils/store.js')
const host = require('../../utils/host.js')
Page({
  data: { on: true, time: '21:00', days: 'everyday', budgetRemind: true, nextText: '' },
  onLoad() {
    const r = store.reminder()
    this.setData({ on: !!r.on, time: r.time || '21:00', days: r.days || 'everyday', budgetRemind: wx.getStorageSync('ledger_remind') !== false })
    this.calcNext()
  },
  onSwitch(e) { this.setData({ on: e.detail.value }); this.calcNext() },
  onTime(e) { this.setData({ time: e.detail.value }); this.calcNext() },
  onDays(e) { this.setData({ days: e.currentTarget.dataset.d }); this.calcNext() },
  onBudgetRemind(e) { this.setData({ budgetRemind: e.detail.value }) },
  // 下一次提醒的时刻（工作日模式跳过周六日）
  calcNext() {
    if (!this.data.on) { this.setData({ nextText: '' }); return }
    const p = this.data.time.split(':')
    const now = new Date()
    for (let i = 0; i < 8; i++) {
      const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + i, +p[0], +p[1])
      if (d <= now) continue
      if (this.data.days === 'workday' && (d.getDay() === 0 || d.getDay() === 6)) continue
      const label = i === 0 ? '今天' : i === 1 ? '明天' : (d.getMonth() + 1) + '月' + d.getDate() + '日 ' + store.WEEK[d.getDay()]
      this.setData({ nextText: '下次提醒：' + label + ' ' + this.data.time })
      return
    }
  },
  onSave() {
    const d = this.data
    const save = () => {
      store.setReminder({ on: d.on, time: d.time, days: d.days })
      wx.setStorageSync('ledger_remind', d.budgetRemind)
      wx.showToast({ title: d.on ? '提醒已开启' : '提醒已关闭', icon: 'success' })
      setTimeout(() => wx.navigateBack(), 600)
    }
    if (!d.on) { save(); return }
    host.subscribe('ledger_daily_remind').then(ok => {
      if (ok) save()
      else wx.showToast({ title: '没有允许通知，提醒无法送达', icon: 'none' })
    })
  }
})
