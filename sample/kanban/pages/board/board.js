const S = require('../../utils/store.js')
Page({
  data: { cols: [], total: 0, overdue: 0, todayText: '' },
  onShow() {
    this.refresh()
  },
  refresh() {
    const list = S.load()
    // 列内：优先级高的在前，同优先级截止早的在前
    const by = (a, b) => b.prio - a.prio || (a.due || '9').localeCompare(b.due || '9') || a.id - b.id
    const cols = S.COLS.map(col => {
      const cards = list.filter(c => c.col === col.key).sort(by).map(S.view)
      return { key: col.key, name: col.name, color: col.color, count: cards.length, cards }
    })
    this.setData({ cols, total: list.length, overdue: list.filter(S.isOverdue).length, todayText: S.today() })
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/card/card?id=' + e.currentTarget.dataset.id })
  },
  // 卡片右上角「⋯」：移到别的列（catchtap，不触发卡片点击）
  onMove(e) {
    const id = Number(e.currentTarget.dataset.id)
    const card = S.find(id)
    if (!card) return
    wx.showActionSheet({
      itemList: S.COLS.map(c => (c.key === card.col ? c.name + '（当前）' : '移到「' + c.name + '」')),
      success: res => {
        const to = S.COLS[res.tapIndex].key
        if (to === card.col) return
        S.update(id, c => { c.col = to })
        wx.showToast({ title: '已移到' + S.COLS[res.tapIndex].name, icon: 'none' })
        this.refresh()
      }
    })
  },
  onAdd(e) {
    const col = e.currentTarget.dataset.col || 'todo'
    wx.navigateTo({ url: '/pages/new/new?col=' + col })
  },
  goStats() { wx.navigateTo({ url: '/pages/stats/stats' }) },
  goMembers() { wx.navigateTo({ url: '/pages/members/members' }) }
})
