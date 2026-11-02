const D = require('../../utils/date.js')
const S = require('../../utils/store.js')
const R = require('../../utils/remind.js')
const HISTORY_KEY = 'cal_search_history'
const HISTORY_MAX = 8
Page({
  data: { kw: '', results: [], history: [], searched: false },
  onShow() {
    // 从详情页改完 / 删掉回来：结果跟着变
    this.setData({ history: wx.getStorageSync(HISTORY_KEY) || [] })
    if (this.data.kw) this.run(this.data.kw)
  },
  run(kw) {
    const q = kw.trim().toLowerCase()
    if (!q) return this.setData({ results: [], searched: false })
    const today = S.today()
    const results = S.loadEvents()
      .filter(ev => (ev.title + '\n' + (ev.note || '')).toLowerCase().indexOf(q) >= 0)
      .map(ev => {
        const next = R.nextOn(ev, today)
        // 不重复且已经过去：显示原日期，排到最后
        const when = next || ev.date
        return {
          id: ev.id,
          title: ev.title,
          note: ev.note || '',
          color: S.COLORS[ev.color].value,
          time: S.timeText(ev),
          repeat: ev.repeat ? S.REPEATS[ev.repeat] : '',
          when,
          whenText: next ? (next === today ? '今天' : D.label(next)) : '已过去 · ' + D.label(ev.date),
          past: !next
        }
      })
      .sort((a, b) => (a.past === b.past ? (a.when < b.when ? -1 : a.when > b.when ? 1 : a.id - b.id) : a.past ? 1 : -1))
    this.setData({ results, searched: true })
  },
  onInput(e) {
    const kw = e.detail.value
    this.setData({ kw })
    this.run(kw)
  },
  // 回车才记进搜索历史（实时过滤的每个中间态不算）
  onConfirm() {
    const kw = this.data.kw.trim()
    if (!kw) return
    const history = [kw].concat(this.data.history.filter(h => h !== kw)).slice(0, HISTORY_MAX)
    wx.setStorageSync(HISTORY_KEY, history)
    this.setData({ history })
  },
  onHistory(e) {
    const kw = e.currentTarget.dataset.kw
    this.setData({ kw })
    this.run(kw)
  },
  onClear() {
    this.setData({ kw: '', results: [], searched: false })
  },
  onClearHistory() {
    wx.showModal({
      title: '清除搜索历史',
      content: '共 ' + this.data.history.length + ' 条',
      confirmText: '清除',
      success: res => {
        if (!res.confirm) return
        wx.removeStorageSync(HISTORY_KEY)
        this.setData({ history: [] })
      }
    })
  },
  onOpen(e) {
    this.onConfirm()
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  }
})
