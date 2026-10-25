const S = require('../../utils/store.js')

const WEEK_STARTS = ['周一', '周日']

Page({
  data: { weekStarts: WEEK_STARTS, wsIdx: 0, reminds: S.REMINDS, remindIdx: 1, count: 0 },
  onShow() {
    const s = S.loadSettings()
    this.setData({ wsIdx: s.weekStart === 1 ? 0 : 1, remindIdx: s.defaultRemind, count: S.loadEvents().length })
  },
  save(patch) {
    S.saveSettings(Object.assign(S.loadSettings(), patch))
  },
  onWeekStart(e) {
    const i = Number(e.detail.value)
    this.setData({ wsIdx: i })
    this.save({ weekStart: i === 0 ? 1 : 0 })
  },
  onRemind(e) {
    const i = Number(e.detail.value)
    this.setData({ remindIdx: i })
    this.save({ defaultRemind: i })
  },
  onClear() {
    wx.showModal({
      title: '清空日程',
      content: '将删除全部 ' + this.data.count + ' 条日程，无法恢复',
      confirmText: '清空',
      confirmColor: '#f2453d',
      success: res => {
        if (!res.confirm) return
        S.saveEvents([])
        this.setData({ count: 0 })
        wx.showToast({ title: '已清空', icon: 'success' })
      }
    })
  }
})
