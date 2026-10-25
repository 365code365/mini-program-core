const S = require('../../utils/store.js')
Page({
  data: {
    kw: '',
    tagId: '',
    tags: [],
    pinned: [],
    others: [],
    total: 0,
    shown: 0,
    preview: true,
    sortName: ''
  },
  onShow() {
    this.refresh()
  },
  refresh() {
    const settings = S.loadSettings()
    const tags = S.loadTags()
    const tagName = {}
    tags.forEach(t => { tagName[t.id] = t.name })
    const kw = this.data.kw.trim().toLowerCase()
    const tagId = tags.some(t => t.id === this.data.tagId) ? this.data.tagId : ''
    const alive = S.loadNotes().filter(n => !n.deleted)
    const hit = alive.filter(n => {
      if (tagId && n.tags.indexOf(tagId) < 0) return false
      if (!kw) return true
      return (n.title + '\n' + n.body).toLowerCase().indexOf(kw) >= 0
    })
    const rows = S.sortNotes(hit, settings.sort).map(n => ({
      id: n.id,
      title: n.title || '无标题',
      body: n.body.replace(/\n/g, ' '),
      bg: S.colorHex(n.color),
      time: S.fmtTime(n.updated),
      pinned: n.pinned,
      tagNames: n.tags.map(t => tagName[t]).filter(Boolean)
    }))
    this.setData({
      tagId,
      tags,
      pinned: rows.filter(r => r.pinned),
      others: rows.filter(r => !r.pinned),
      total: alive.length,
      shown: rows.length,
      preview: settings.preview,
      sortName: S.SORTS[settings.sort]
    })
  },
  // 搜索框实时过滤：每次 input 都重算
  onSearch(e) {
    this.setData({ kw: e.detail.value })
    this.refresh()
  },
  onClearSearch() {
    this.setData({ kw: '' })
    this.refresh()
  },
  onChip(e) {
    this.setData({ tagId: e.currentTarget.dataset.id })
    this.refresh()
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/edit/edit?id=' + e.currentTarget.dataset.id })
  },
  onNew() {
    wx.navigateTo({ url: '/pages/edit/edit' })
  },
  // 长按：置顶 / 取消置顶、移到回收站
  onLong(e) {
    const id = Number(e.currentTarget.dataset.id)
    const list = S.loadNotes()
    const n = list.find(i => i.id === id)
    if (!n) return
    wx.showActionSheet({
      itemList: [n.pinned ? '取消置顶' : '置顶', '移到回收站'],
      success: res => {
        if (res.tapIndex === 0) {
          n.pinned = !n.pinned
          wx.showToast({ title: n.pinned ? '已置顶' : '已取消置顶', icon: 'none' })
        } else if (res.tapIndex === 1) {
          n.deleted = true
          n.pinned = false
          wx.showToast({ title: '已移到回收站', icon: 'none' })
        }
        S.saveNotes(list)
        this.refresh()
      }
    })
  },
  goTrash() { wx.navigateTo({ url: '/pages/trash/trash' }) },
  goTags() { wx.navigateTo({ url: '/pages/tags/tags' }) },
  goSettings() { wx.navigateTo({ url: '/pages/settings/settings' }) }
})
