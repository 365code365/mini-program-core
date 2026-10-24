const S = require('../../utils/store.js')

Page({
  data: {
    settings: S.loadSettings(),
    sliders: [
      { key: 'focus', label: '专注时长', unit: '分钟', min: 10, max: 60, step: 5 },
      { key: 'short', label: '短休息', unit: '分钟', min: 3, max: 15, step: 1 },
      { key: 'long', label: '长休息', unit: '分钟', min: 10, max: 30, step: 5 },
      { key: 'interval', label: '长休息间隔', unit: '个番茄', min: 2, max: 6, step: 1 },
      { key: 'goal', label: '每日目标', unit: '个番茄', min: 2, max: 12, step: 1 }
    ],
    switches: [
      { key: 'sound', label: '结束提示', desc: '一轮结束时振动提醒' },
      { key: 'autoBreak', label: '自动开始休息', desc: '专注结束后直接进入休息计时' },
      { key: 'daily', label: '每日计划提醒', desc: '每天 08:30 提醒你排今天的待办' }
    ],
    streak: 0,
    totalHours: 0,
    sessionCount: 0
  },

  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 3 })
    const sessions = S.loadSessions()
    const days = {}
    sessions.forEach(s => { days[s.date] = 1 })
    this.setData({
      settings: S.loadSettings(),
      streak: Object.keys(days).length,
      totalHours: (sessions.reduce((m, s) => m + s.minutes, 0) / 60).toFixed(1),
      sessionCount: sessions.length
    })
  },

  onSlide(e) {
    this.setData({ ['settings.' + e.currentTarget.dataset.k]: e.detail.value })
  },

  onSlideEnd(e) {
    this.onSlide(e)
    S.saveSettings(this.data.settings)
  },

  onSwitch(e) {
    this.setData({ ['settings.' + e.currentTarget.dataset.k]: e.detail.value })
    S.saveSettings(this.data.settings)
    wx.showToast({ title: e.detail.value ? '已开启' : '已关闭', icon: 'none' })
  },

  onExport() {
    const s = S.loadSessions()
    const text = s.map(i => i.date + ' ' + i.at + ' ' + i.title + ' ' + i.minutes + '分钟').join('\n')
    wx.setClipboardData({
      data: text,
      success: () => wx.showToast({ title: '已复制到剪贴板', icon: 'none' })
    })
  },

  onReset() {
    wx.showModal({
      title: '恢复默认设置',
      content: '专注 25 分钟 / 短休息 5 分钟 / 长休息 15 分钟',
      confirmColor: '#ff6250',
      success: res => {
        if (!res.confirm) return
        wx.removeStorageSync('ft_settings')
        this.onShow()
      }
    })
  },

  onClear() {
    wx.showActionSheet({
      itemList: ['清空专注记录', '清空全部待办和记录'],
      itemColor: '#f2453d',
      success: res => {
        wx.removeStorageSync('ft_sessions')
        if (res.tapIndex === 1) wx.removeStorageSync('ft_todos')
        wx.showToast({ title: '已清空', icon: 'success' })
        this.onShow()
      }
    })
  }
})
