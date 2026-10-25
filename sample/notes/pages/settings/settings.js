const S = require('../../utils/store.js')
Page({
  data: { sorts: S.SORTS, sort: 0, preview: true, rawSort: null, stats: '' },
  onShow() {
    const s = S.loadSettings()
    const notes = S.loadNotes()
    const alive = notes.filter(n => !n.deleted)
    const words = alive.reduce((m, n) => m + S.wordCount(n.body), 0)
    this.setData({ sort: s.sort, preview: s.preview, stats: alive.length + ' 篇笔记 · ' + words + ' 字 · 回收站 ' + (notes.length - alive.length) + ' 篇' })
  },
  // picker selector：e.detail.value 是下标（number）
  onSort(e) {
    const v = e.detail.value
    const s = S.loadSettings()
    s.sort = Number(v)
    S.saveSettings(s)
    this.setData({ sort: s.sort, rawSort: v })
  },
  onPreview(e) {
    const s = S.loadSettings()
    s.preview = e.detail.value
    S.saveSettings(s)
    this.setData({ preview: s.preview })
  }
})
