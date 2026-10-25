const S = require('../../utils/store.js')
Page({
  data: {
    card: null,
    colName: '',
    prioNames: S.PRIOS.map(p => p.name),
    ownerNames: S.MEMBERS.map(m => m.name),
    ownerIdx: 0,
    prioColor: '',
    overdue: false,
    doneIds: [],
    pct: 0,
    progressText: '',
    subDraft: '',
    raw: {}
  },
  onLoad(query) {
    this.id = Number(query.id)
    this.reload()
  },
  reload() {
    const c = S.find(this.id)
    if (!c) {
      this.setData({ card: null })
      return
    }
    const p = S.progress(c)
    const col = S.COLS.find(i => i.key === c.col)
    this.setData({
      card: c,
      colName: col ? col.name : '',
      ownerIdx: Math.max(0, S.MEMBERS.findIndex(m => m.id === c.owner)),
      prioColor: S.PRIOS[c.prio].color,
      overdue: S.isOverdue(c),
      pct: p.pct,
      progressText: p.done + '/' + p.total
    })
  },
  patch(fn) {
    S.update(this.id, fn)
    this.reload()
  },
  onTitle(e) {
    const v = e.detail.value
    this.patch(c => { c.title = v })
  },
  onDesc(e) {
    const v = e.detail.value
    this.patch(c => { c.desc = v })
  },
  // picker selector：e.detail.value 是下标（number）
  onPrio(e) {
    const v = e.detail.value
    this.setData({ 'raw.prio': v })
    this.patch(c => { c.prio = Number(v) })
  },
  onOwner(e) {
    const v = e.detail.value
    this.setData({ 'raw.owner': v })
    this.patch(c => { c.owner = S.MEMBERS[Number(v)].id })
  },
  // picker date：e.detail.value 是 'YYYY-MM-DD' 字符串
  onDue(e) {
    const v = e.detail.value
    this.setData({ 'raw.due': v })
    this.patch(c => { c.due = v })
  },
  // checkbox-group：选中的子任务 id（字符串数组）
  onSubs(e) {
    const ids = e.detail.value.map(Number)
    this.patch(c => { c.subs.forEach(s => { s.done = ids.indexOf(s.id) >= 0 }) })
    const p = S.progress(S.find(this.id))
    if (p.total && p.done === p.total) wx.showToast({ title: '子任务全部完成', icon: 'success' })
  },
  onSubDraft(e) {
    this.setData({ subDraft: e.detail.value })
  },
  onSubAdd() {
    const text = this.data.subDraft.trim()
    if (!text) {
      wx.showToast({ title: '写点子任务内容', icon: 'none' })
      return
    }
    this.patch(c => { c.subs.push({ id: c.subs.reduce((m, s) => Math.max(m, s.id), 0) + 1, text, done: false }) })
    this.setData({ subDraft: '' })
  },
  onMove() {
    const c = this.data.card
    wx.showActionSheet({
      itemList: S.COLS.map(i => (i.key === c.col ? i.name + '（当前）' : '移到「' + i.name + '」')),
      success: res => {
        const to = S.COLS[res.tapIndex].key
        if (to === c.col) return
        this.patch(x => { x.col = to })
        wx.showToast({ title: '已移到' + S.COLS[res.tapIndex].name, icon: 'none' })
      }
    })
  },
  onDelete() {
    wx.showModal({
      title: '删除卡片',
      content: '「' + this.data.card.title + '」删除后无法恢复',
      confirmText: '删除',
      confirmColor: '#f5222d',
      success: res => {
        if (!res.confirm) return
        S.save(S.load().filter(c => c.id !== this.id))
        wx.navigateBack()
      }
    })
  }
})
