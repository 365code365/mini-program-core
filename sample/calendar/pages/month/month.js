const D = require('../../utils/date.js')
const S = require('../../utils/store.js')

Page({
  data: {
    year: 0,
    month: 0,
    title: '',
    heads: [],
    cells: [],
    today: '',
    selDate: '',
    selLabel: '',
    selEvents: [],
    monthCount: 0
  },
  onLoad(query) {
    // ?today=2026-03-15：把「今天」钉死（测试 / 演示用）
    if (query.today && /^\d{4}-\d{2}-\d{2}$/.test(query.today)) getApp().globalData.today = query.today
    const t = S.today()
    this.setData({ today: t, year: Number(t.slice(0, 4)), month: Number(t.slice(5, 7)), selDate: t })
  },
  onShow() {
    // 从新建 / 编辑 / 设置页回来：日程与周起始日都可能变了
    this.build()
  },
  build() {
    const { year, month, selDate, today } = this.data
    const settings = S.loadSettings()
    const events = S.loadEvents()
    let monthCount = 0
    const cells = D.monthGrid(year, month, settings.weekStart).map(c => {
      const evs = S.eventsOn(events, c.date)
      if (c.cur && evs.length) monthCount++
      return {
        date: c.date,
        day: c.day,
        cur: c.cur,
        isToday: c.date === today,
        sel: c.date === selDate,
        // 最多三个圆点，颜色取日程颜色
        dots: evs.slice(0, 3).map(e => e.color)
      }
    })
    this.setData({
      title: year + '年' + month + '月',
      heads: D.weekHeads(settings.weekStart),
      cells,
      monthCount,
      selLabel: D.longLabel(selDate) + (selDate === today ? ' · 今天' : ''),
      selEvents: S.eventsOn(events, selDate)
    })
  },
  goMonth(n) {
    const m = D.addMonths(this.data.year, this.data.month, n)
    // 切月后选中该月 1 号；切回今天所在月时选中今天
    const sel = this.data.today.slice(0, 7) === D.fmt(m.year, m.month, 1).slice(0, 7) ? this.data.today : D.fmt(m.year, m.month, 1)
    this.setData({ year: m.year, month: m.month, selDate: sel })
    this.build()
  },
  onPrev() { this.goMonth(-1) },
  onNext() { this.goMonth(1) },
  onToday() {
    const t = this.data.today
    this.setData({ year: Number(t.slice(0, 4)), month: Number(t.slice(5, 7)), selDate: t })
    this.build()
  },
  onCell(e) {
    const date = e.currentTarget.dataset.date
    const m = Number(date.slice(5, 7))
    // 点到补位格：跳到那个月并选中它
    if (m !== this.data.month) this.setData({ year: Number(date.slice(0, 4)), month: m })
    this.setData({ selDate: date })
    this.build()
  },
  // 左右滑动切月：手指横向移动超过 60px 且横向为主
  onTouchStart(e) {
    const t = e.touches[0]
    this.touch = { x: t.clientX, y: t.clientY }
  },
  onTouchEnd(e) {
    if (!this.touch) return
    const t = e.changedTouches[0]
    const dx = t.clientX - this.touch.x
    const dy = t.clientY - this.touch.y
    this.touch = null
    if (Math.abs(dx) < 60 || Math.abs(dx) < Math.abs(dy)) return
    this.goMonth(dx < 0 ? 1 : -1)
  },
  onAdd() {
    wx.navigateTo({ url: '/pages/edit/edit?date=' + this.data.selDate })
  },
  onEvent(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  },
  onList() {
    wx.navigateTo({ url: '/pages/list/list' })
  },
  onSettings() {
    wx.navigateTo({ url: '/pages/settings/settings' })
  }
})
