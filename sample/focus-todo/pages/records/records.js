const S = require('../../utils/store.js')
const RANGES = [
  { text: '近 7 天', days: 7 },
  { text: '近 30 天', days: 30 },
  { text: '全部', days: 0 }
]
// 专注记录：按天分组、新的在上；长按删除（关联任务的番茄数跟着减）；右上「补录」
Page({
  data: { ranges: RANGES, range: 0, groups: [], count: 0, minutes: 0 },
  onShow() {
    this.refresh()
  },
  refresh() {
    const days = RANGES[this.data.range].days
    const from = days ? S.dayOffset(-(days - 1)) : ''
    // 记下在原数组里的下标，删除时按它删（同一分钟可能有两条一样的）
    const all = S.loadSessions().map((s, k) => Object.assign({ k }, s)).filter(s => !from || s.date >= from)
    all.sort((a, b) => (a.date === b.date ? (a.at < b.at ? 1 : a.at > b.at ? -1 : b.k - a.k) : a.date < b.date ? 1 : -1))
    const groups = []
    all.forEach(s => {
      let g = groups[groups.length - 1]
      if (!g || g.date !== s.date) {
        g = { date: s.date, label: S.label(s.date), items: [], minutes: 0 }
        groups.push(g)
      }
      g.items.push({ k: s.k, at: s.at, title: s.title, minutes: s.minutes, manual: !!s.manual })
      g.minutes += s.minutes
    })
    this.setData({ groups, count: all.length, minutes: all.reduce((m, s) => m + s.minutes, 0) })
  },
  onRange(e) {
    this.setData({ range: Number(e.currentTarget.dataset.i) })
    this.refresh()
  },
  onAdd() {
    wx.navigateTo({ url: '/pages/record-add/record-add' })
  },
  onLong(e) {
    const k = Number(e.currentTarget.dataset.k)
    const s = S.loadSessions()[k]
    if (!s) return
    wx.showModal({
      title: '删除这条记录',
      content: s.date.slice(5) + ' ' + s.at + ' ' + s.title + ' · ' + s.minutes + ' 分钟',
      confirmText: '删除',
      confirmColor: '#f2453d',
      success: res => {
        if (!res.confirm) return
        const list = S.loadSessions()
        const gone = list.splice(k, 1)[0]
        S.saveSessions(list)
        // 关联了任务的：任务上的番茄数减一
        if (gone && gone.taskId) {
          const todos = S.loadTodos()
          const it = todos.find(i => i.id === gone.taskId)
          if (it && it.pomos > 0) {
            it.pomos--
            S.saveTodos(todos)
          }
        }
        wx.showToast({ title: '已删除', icon: 'none' })
        this.refresh()
      }
    })
  }
})
