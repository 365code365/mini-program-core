const S = require('../../utils/store.js')
Page({
  data: { items: [] },
  onShow() {
    this.refresh()
  },
  refresh() {
    const items = S.loadNotes().filter(n => n.deleted).map(n => ({
      id: n.id, title: n.title || '无标题', body: n.body.replace(/\n/g, ' '), time: S.fmtTime(n.updated)
    }))
    this.setData({ items })
  },
  onRestore(e) {
    const id = Number(e.currentTarget.dataset.id)
    const list = S.loadNotes()
    const n = list.find(i => i.id === id)
    if (!n) return
    n.deleted = false
    S.saveNotes(list)
    wx.showToast({ title: '已恢复', icon: 'success' })
    this.refresh()
  },
  // 彻底删除要二次确认
  onPurge(e) {
    const id = Number(e.currentTarget.dataset.id)
    const n = S.loadNotes().find(i => i.id === id)
    if (!n) return
    wx.showModal({
      title: '彻底删除',
      content: '「' + (n.title || '无标题') + '」删除后无法恢复',
      confirmText: '删除',
      confirmColor: '#e5483d',
      success: res => {
        if (!res.confirm) return
        S.saveNotes(S.loadNotes().filter(i => i.id !== id))
        this.refresh()
      }
    })
  },
  onClear() {
    if (!this.data.items.length) return
    wx.showModal({
      title: '清空回收站',
      content: '将彻底删除 ' + this.data.items.length + ' 篇笔记',
      confirmText: '清空',
      confirmColor: '#e5483d',
      success: res => {
        if (!res.confirm) return
        S.saveNotes(S.loadNotes().filter(i => !i.deleted))
        wx.showToast({ title: '已清空', icon: 'none' })
        this.refresh()
      }
    })
  }
})
