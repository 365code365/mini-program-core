const S = require('../../utils/store.js')
Page({
  data: { members: [], selected: '', cards: [], selName: '' },
  onShow() {
    this.refresh()
  },
  refresh() {
    const list = S.load()
    const members = S.MEMBERS.map(m => {
      const mine = list.filter(c => c.owner === m.id)
      return {
        id: m.id, name: m.name, role: m.role, color: m.color, initial: m.name.slice(-1),
        total: mine.length,
        open: mine.filter(c => c.col !== 'done').length,
        late: mine.filter(S.isOverdue).length
      }
    })
    // 任务多的排前面
    members.sort((a, b) => b.open - a.open || b.total - a.total)
    this.setData({ members })
    this.pick(this.data.selected)
  },
  pick(id) {
    const m = S.MEMBERS.find(i => i.id === id)
    const cards = m ? S.load().filter(c => c.owner === id).map(S.view) : []
    this.setData({ selected: m ? id : '', selName: m ? m.name : '', cards })
  },
  // 再点一次同一个人：收起
  onMember(e) {
    const id = e.currentTarget.dataset.id
    this.pick(id === this.data.selected ? '' : id)
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/card/card?id=' + e.currentTarget.dataset.id })
  }
})
