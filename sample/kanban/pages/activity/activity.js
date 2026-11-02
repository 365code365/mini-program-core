const S = require('../../utils/store.js')
const TYPE_COLOR = { create: '#2f54eb', move: '#13c2c2', edit: '#fa8c16', comment: '#722ed1', delete: '#f5222d' }
// 团队动态：按天分组、最新在上；可按类型筛选
Page({
  data: { types: [], type: '', groups: [], total: 0 },
  onShow() {
    this.refresh()
  },
  refresh() {
    const all = S.loadLog()
    const type = this.data.type
    const alive = {}
    S.load().forEach(c => { alive[c.id] = true })
    const today = S.today()
    const yesterday = S.dayOffset(-1)
    const groups = []
    let cur = null
    all.filter(l => !type || l.type === type).slice().reverse().forEach(l => {
      const day = S.fmt(new Date(l.at))
      if (!cur || cur.day !== day) {
        cur = { day, label: day === today ? '今天' : day === yesterday ? '昨天' : day.slice(5), items: [] }
        groups.push(cur)
      }
      const who = S.member(l.who)
      cur.items.push({
        id: l.id, cardId: l.cardId, title: l.title, text: l.text, who: who.name, color: who.color, initial: who.name.slice(-1),
        time: S.timeText(l.at).slice(6), dot: TYPE_COLOR[l.type] || '#8c8c8c', gone: !alive[l.cardId]
      })
    })
    const types = [{ key: '', name: '全部', count: all.length }].concat(S.LOG_TYPES.map(t => ({ key: t.key, name: t.name, count: all.filter(l => l.type === t.key).length })))
    this.setData({ types, groups, total: groups.reduce((m, g) => m + g.items.length, 0) })
  },
  onType(e) {
    this.setData({ type: e.currentTarget.dataset.key })
    this.refresh()
  },
  onOpen(e) {
    const it = e.currentTarget.dataset
    if (it.gone) return wx.showToast({ title: '这张卡片已被删除', icon: 'none' })
    wx.navigateTo({ url: '/pages/card/card?id=' + it.id })
  }
})
