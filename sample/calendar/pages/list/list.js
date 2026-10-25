const D = require('../../utils/date.js')
const S = require('../../utils/store.js')

// 未来 N 天（含今天）的日程，重复规则逐日展开后按日期分组
const DAYS = 30

Page({
  data: { groups: [], total: 0, range: '' },
  onShow() {
    const today = S.today()
    const events = S.loadEvents()
    const groups = []
    let total = 0
    for (let i = 0; i < DAYS; i++) {
      const date = D.addDays(today, i)
      const items = S.eventsOn(events, date)
      if (!items.length) continue
      total += items.length
      groups.push({ date, label: i === 0 ? '今天' : i === 1 ? '明天' : D.label(date), rel: i === 0 ? '' : i + ' 天后', items })
    }
    this.setData({ groups, total, range: today.slice(5) + ' ~ ' + D.addDays(today, DAYS - 1).slice(5) })
  },
  onEvent(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  }
})
