const S = require('../../utils/store.js')
Page({
  data: { tags: [], name: '', error: '' },
  onShow() {
    this.refresh()
  },
  refresh() {
    const notes = S.loadNotes().filter(n => !n.deleted)
    const tags = S.loadTags().map(t => ({ id: t.id, name: t.name, count: notes.filter(n => n.tags.indexOf(t.id) >= 0).length }))
    this.setData({ tags })
  },
  onInput(e) {
    this.setData({ name: e.detail.value, error: '' })
  },
  // 新增：去掉首尾空白、不能为空、不能与已有标签重名（忽略大小写）、最多 8 个字
  onAdd() {
    const name = this.data.name.trim()
    let error = ''
    const tags = S.loadTags()
    if (!name) error = '请输入标签名'
    else if (name.length > 8) error = '标签名最多 8 个字'
    else if (tags.some(t => t.name.toLowerCase() === name.toLowerCase())) error = '标签「' + name + '」已存在'
    if (error) {
      this.setData({ error })
      wx.showToast({ title: error, icon: 'none' })
      return
    }
    const id = 't' + (tags.reduce((m, t) => Math.max(m, Number(t.id.slice(1)) || 0), 0) + 1)
    tags.push({ id, name })
    S.saveTags(tags)
    this.setData({ name: '', error: '' })
    wx.showToast({ title: '已添加', icon: 'success' })
    this.refresh()
  },
  // 删除标签：同时从所有笔记上摘掉
  onDel(e) {
    const id = e.currentTarget.dataset.id
    const t = this.data.tags.find(i => i.id === id)
    if (!t) return
    wx.showModal({
      title: '删除标签',
      content: '删除「' + t.name + '」，' + t.count + ' 篇笔记会去掉这个标签',
      success: res => {
        if (!res.confirm) return
        S.saveTags(S.loadTags().filter(i => i.id !== id))
        const notes = S.loadNotes()
        notes.forEach(n => { n.tags = n.tags.filter(x => x !== id) })
        S.saveNotes(notes)
        this.refresh()
      }
    })
  }
})
