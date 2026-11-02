const S = require('../../utils/store.js')
Page({
  data: { items: [] },
  onShow() {
    this.refresh()
  },
  refresh() {
    // 超过 30 天的自动彻底删除
    const expired = S.purgeExpired()
    if (expired) wx.showToast({ title: '已自动清理 ' + expired + ' 篇过期笔记', icon: 'none' })
    const items = S.loadNotes().filter(n => n.deleted).map(n => {
      const left = S.trashDaysLeft(n)
      return {
        id: n.id, title: n.title || '无标题', body: n.body.replace(/\n/g, ' '), time: S.fmtTime(n.deletedAt || n.updated),
        left, leftText: left <= 1 ? '明天自动删除' : left + ' 天后自动删除'
      }
    })
    this.setData({ items })
  },
  onRestore(e) {
    const id = Number(e.currentTarget.dataset.id)
    const list = S.loadNotes()
    const n = list.find(i => i.id === id)
    if (!n) return
    n.deleted = false
    delete n.deletedAt
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
        S.dropVersions([id])
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
        S.dropVersions(this.data.items.map(i => i.id))
        S.saveNotes(S.loadNotes().filter(i => !i.deleted))
        wx.showToast({ title: '已清空', icon: 'none' })
        this.refresh()
      }
    })
  }
})
