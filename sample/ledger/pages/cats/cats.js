const store = require('../../utils/store.js')
Page({
  data: { type: 'out', list: [], name: '', preview: '新', color: store.PALETTE[0], palette: store.PALETTE, formSeq: 0 },
  onLoad(q) { if (q && q.type === 'in') this.setData({ type: 'in' }) },
  onShow() { this.refresh() },
  refresh() {
    const t = this.data.type
    const list = store.cats(t).map(c => ({ name: c.name, ch: c.ch, color: c.color, custom: !!c.custom, count: store.catCount(t, c.name) }))
    this.setData({ list: list })
  },
  onType(e) {
    const t = e.currentTarget.dataset.t
    if (t === this.data.type) return
    this.setData({ type: t })
    this.refresh()
  },
  onName(e) { const v = e.detail.value; this.setData({ name: v, preview: v.trim().charAt(0) || '新' }) },
  onColor(e) { this.setData({ color: e.currentTarget.dataset.c }) },
  onAdd() {
    const err = store.addCat(this.data.type, this.data.name, this.data.color)
    if (err) { wx.showToast({ title: err, icon: 'none' }); return }
    wx.showToast({ title: '已添加「' + this.data.name.trim() + '」', icon: 'none' })
    this.setData({ name: '', preview: '新', formSeq: this.data.formSeq + 1 })
    this.refresh()
  },
  onDelete(e) {
    const name = String(e.currentTarget.dataset.name)
    const c = this.data.list.filter(x => x.name === name)[0]
    wx.showModal({
      title: '删除分类「' + name + '」？',
      content: c.count ? '这个分类下的 ' + c.count + ' 笔账单会归入「其他」' : '这个分类还没有账单',
      confirmText: '删除',
      confirmColor: '#f04848',
      success: (res) => {
        if (!res.confirm) return
        store.removeCat(this.data.type, name)
        wx.showToast({ title: '已删除', icon: 'success' })
        this.refresh()
      }
    })
  }
})
