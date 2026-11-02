const S = require('../../utils/store.js')
// 筛选：关键词（标题 / 描述 / 子任务）+ 负责人 + 优先级 + 只看逾期 / 未完成，条件之间是「且」
const FILTER_KEY = 'kb_filter'
Page({
  data: {
    kw: '',
    owners: [],
    prios: [],
    owner: '',
    prio: -1,
    lateOnly: false,
    openOnly: false,
    results: [],
    active: 0
  },
  onLoad() {
    // 上次的筛选条件留着（关键词不留）
    const f = wx.getStorageSync(FILTER_KEY) || {}
    this.setData({
      owner: f.owner || '', prio: typeof f.prio === 'number' ? f.prio : -1, lateOnly: !!f.lateOnly, openOnly: !!f.openOnly,
      owners: S.MEMBERS.map(m => ({ id: m.id, name: m.name, color: m.color })),
      prios: S.PRIOS.map((p, i) => ({ i, name: p.name, color: p.color }))
    })
  },
  onShow() {
    this.run()
  },
  run() {
    const { kw, owner, prio, lateOnly, openOnly } = this.data
    const q = kw.trim().toLowerCase()
    const by = (a, b) => b.prio - a.prio || (a.due || '9').localeCompare(b.due || '9') || a.id - b.id
    const results = S.load().filter(c => {
      if (owner && c.owner !== owner) return false
      if (prio >= 0 && c.prio !== prio) return false
      if (lateOnly && !S.isOverdue(c)) return false
      if (openOnly && c.col === 'done') return false
      if (!q) return true
      const hay = [c.title, c.desc || ''].concat(c.subs.map(s => s.text)).join('\n').toLowerCase()
      return hay.indexOf(q) >= 0
    }).sort(by).map(c => Object.assign(S.view(c), { colName: S.colName(c.col) }))
    const active = (owner ? 1 : 0) + (prio >= 0 ? 1 : 0) + (lateOnly ? 1 : 0) + (openOnly ? 1 : 0)
    this.setData({ results, active })
    wx.setStorageSync(FILTER_KEY, { owner, prio, lateOnly, openOnly })
  },
  onKw(e) {
    this.setData({ kw: e.detail.value })
    this.run()
  },
  // 再点一次已选中的同一项：取消
  onOwner(e) {
    const id = e.currentTarget.dataset.id
    this.setData({ owner: this.data.owner === id ? '' : id })
    this.run()
  },
  onPrio(e) {
    const i = Number(e.currentTarget.dataset.i)
    this.setData({ prio: this.data.prio === i ? -1 : i })
    this.run()
  },
  onLate() {
    this.setData({ lateOnly: !this.data.lateOnly })
    this.run()
  },
  onOpenOnly() {
    this.setData({ openOnly: !this.data.openOnly })
    this.run()
  },
  onReset() {
    this.setData({ kw: '', owner: '', prio: -1, lateOnly: false, openOnly: false })
    this.run()
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/card/card?id=' + e.currentTarget.dataset.id })
  }
})
