const S = require('../../utils/store.js')
// 一篇笔记的历史版本：每次打开编辑并改动后，改之前的内容存成一个版本（最多 10 个）
Page({
  data: { id: 0, title: '', current: null, versions: [], open: -1, missing: false },
  onLoad(query) {
    this.setData({ id: Number(query.id) })
  },
  onShow() {
    this.refresh()
  },
  refresh() {
    const n = S.loadNotes().find(i => i.id === this.data.id && !i.deleted)
    if (!n) return this.setData({ missing: true, versions: [] })
    const versions = S.loadVersions(n.id).map((v, i) => ({
      i,
      title: v.title || '无标题',
      body: v.body,
      time: S.fmtTime(v.ts),
      words: S.wordCount(v.body),
      // 与当前正文相比多 / 少了多少字
      delta: S.wordCount(n.body) - S.wordCount(v.body)
    }))
    this.setData({
      missing: false,
      title: n.title || '无标题',
      current: { time: S.fmtTime(n.updated), words: S.wordCount(n.body) },
      versions
    })
  },
  onToggle(e) {
    const i = Number(e.currentTarget.dataset.i)
    this.setData({ open: this.data.open === i ? -1 : i })
  },
  // 恢复：当前内容先存成一个新版本，再用选中的版本覆盖（恢复本身也能撤回）
  onRestore(e) {
    const i = Number(e.currentTarget.dataset.i)
    const v = S.loadVersions(this.data.id)[i]
    if (!v) return
    wx.showModal({
      title: '恢复到这个版本',
      content: '当前内容会先保存为一个历史版本，可以随时再恢复回来',
      confirmText: '恢复',
      success: res => {
        if (!res.confirm) return
        const list = S.loadNotes()
        const n = list.find(x => x.id === this.data.id)
        if (!n) return
        S.pushVersion(n.id, n.title, n.body)
        n.title = v.title
        n.body = v.body
        n.updated = Date.now()
        S.saveNotes(list)
        this.setData({ open: -1 })
        wx.showToast({ title: '已恢复', icon: 'success' })
        this.refresh()
      }
    })
  },
  onClearAll() {
    if (!this.data.versions.length) return
    wx.showModal({
      title: '清除历史版本',
      content: '删除这篇笔记的 ' + this.data.versions.length + ' 个历史版本，当前内容不受影响',
      confirmText: '清除',
      confirmColor: '#e5483d',
      success: res => {
        if (!res.confirm) return
        S.dropVersions([this.data.id])
        this.refresh()
      }
    })
  }
})
