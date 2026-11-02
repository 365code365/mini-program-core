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
  // 导出成 JSON（去掉 id，导入时重新分配），带版本号便于以后兼容
  onExport() {
    const list = S.loadEvents()
    if (!list.length) return wx.showToast({ title: '没有可导出的日程', icon: 'none' })
    const events = list.map(e => {
      const o = Object.assign({}, e)
      delete o.id
      return o
    })
    wx.setClipboardData({
      data: JSON.stringify({ app: 'calendar', v: 1, events }),
      success: () => wx.showToast({ title: '已复制 ' + list.length + ' 条', icon: 'none' })
    })
  },
  onImport() {
    wx.getClipboardData({
      success: res => {
        let events = null
        try {
          const o = JSON.parse(res.data)
          events = o && o.app === 'calendar' && Array.isArray(o.events) ? o.events : null
        } catch (e) {
          events = null
        }
        if (!events) {
          wx.showModal({ title: '无法导入', content: '剪贴板里不是日程本导出的内容。先在旧设备上点「导出日程」再来导入。', showCancel: false })
          return
        }
        const ok = events.filter(S.validEvent)
        if (!ok.length) return wx.showToast({ title: '没有可导入的日程', icon: 'none' })
        const list = S.loadEvents()
        let id = list.reduce((m, i) => Math.max(m, i.id), 0)
        ok.forEach(e => {
          id++
          list.push({ id, title: e.title.trim(), allDay: !!e.allDay, start: e.start || '09:00', end: e.end || '10:00', date: e.date,
            repeat: e.repeat, remind: e.remind, color: e.color, note: String(e.note || '').slice(0, 60) })
        })
        S.saveEvents(list)
        this.setData({ count: list.length })
        const skip = events.length - ok.length
        wx.showToast({ title: '导入 ' + ok.length + ' 条' + (skip ? '，跳过 ' + skip + ' 条' : ''), icon: 'none' })
      },
      fail: () => wx.showToast({ title: '读取剪贴板失败', icon: 'none' })
    })
  },
  onRestore() {
    wx.showModal({
      title: '恢复示例日程',
      content: '会用示例数据替换现有的 ' + this.data.count + ' 条日程',
      confirmText: '恢复',
      success: res => {
        if (!res.confirm) return
        const list = S.seed()
        S.saveEvents(list)
        this.setData({ count: list.length })
        wx.showToast({ title: '已恢复', icon: 'success' })
      }
    })
  },
  onClear() {
    if (!this.data.count) return wx.showToast({ title: '已经没有日程了', icon: 'none' })
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
