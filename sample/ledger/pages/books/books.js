const store = require('../../utils/store.js')
Page({
  data: { list: [], curId: '', panel: false, editId: '', name: '', color: '', palette: store.PALETTE, formSeq: 0 },
  onShow() { this.refresh() },
  refresh() {
    const now = new Date()
    const prefix = now.getFullYear() + '-' + store.pad(now.getMonth() + 1) + '-'
    const recs = store.raw()
    const list = store.books().map(b => {
      const mine = recs.filter(r => (r.book || 'b1') === b.id)
      const month = mine.filter(r => r.date.indexOf(prefix) === 0)
      return { id: b.id, name: b.name, color: b.color, ch: b.name.charAt(0), count: mine.length, outText: store.money(store.sum(month, 'out')), canDel: b.id !== 'b1' }
    })
    this.setData({ list: list, curId: store.curBookId() })
  },
  onPick(e) {
    const id = String(e.currentTarget.dataset.id)
    const b = this.data.list.filter(x => x.id === id)[0]
    if (!b) return
    if (id === this.data.curId) { wx.navigateBack(); return }
    store.setCurBook(id)
    this.setData({ curId: id })
    wx.showToast({ title: '已切换到「' + b.name + '」', icon: 'none' })
    setTimeout(() => wx.navigateBack(), 500)
  },
  onNew() { this.setData({ panel: true, editId: '', name: '', color: store.PALETTE[this.data.list.length % store.PALETTE.length], formSeq: this.data.formSeq + 1 }) },
  onEdit(e) {
    const id = String(e.currentTarget.dataset.id)
    const b = this.data.list.filter(x => x.id === id)[0]
    this.setData({ panel: true, editId: id, name: b.name, color: b.color, formSeq: this.data.formSeq + 1 })
  },
  onClose() { this.setData({ panel: false }) },
  noop() {},
  onName(e) { this.setData({ name: e.detail.value }) },
  onColor(e) { this.setData({ color: e.currentTarget.dataset.c }) },
  onSave() {
    const err = store.saveBook(this.data.editId, this.data.name, this.data.color)
    if (err) { wx.showToast({ title: err, icon: 'none' }); return }
    wx.showToast({ title: this.data.editId ? '已保存' : '账本已创建', icon: 'success' })
    this.setData({ panel: false })
    this.refresh()
  },
  onDelete(e) {
    const id = String(e.currentTarget.dataset.id)
    const b = this.data.list.filter(x => x.id === id)[0]
    wx.showModal({
      title: '删除「' + b.name + '」？',
      content: b.count ? '账本里的 ' + b.count + ' 笔账单会一起删除，无法恢复' : '这个账本还没有账单',
      confirmText: '删除',
      confirmColor: '#f04848',
      success: (res) => {
        if (!res.confirm) return
        store.removeBook(id)
        wx.showToast({ title: '已删除', icon: 'success' })
        this.refresh()
      }
    })
  }
})
