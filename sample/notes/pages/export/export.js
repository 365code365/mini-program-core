const S = require('../../utils/store.js')
const FORMATS = [
  { key: 'text', name: '纯文本' },
  { key: 'md', name: 'Markdown' },
  { key: 'json', name: '备份（可导入）' }
]
const PREVIEW_MAX = 600
Page({
  data: { notes: [], selected: [], formats: FORMATS, format: 'md', preview: '', size: 0 },
  onShow() {
    const tags = {}
    S.loadTags().forEach(t => { tags[t.id] = t.name })
    this.tagName = tags
    const alive = S.sortNotes(S.loadNotes().filter(n => !n.deleted), 0)
    // 默认全选；回来时保留之前的勾选（已经不存在的去掉）
    const ids = alive.map(n => String(n.id))
    const prev = this.data.notes.length ? this.data.selected.filter(i => ids.indexOf(i) >= 0) : ids
    this.setData({
      notes: alive.map(n => ({ id: String(n.id), title: n.title || '无标题', time: S.fmtTime(n.updated), on: prev.indexOf(String(n.id)) >= 0 })),
      selected: prev
    })
    this.build()
  },
  picked() {
    const ids = this.data.selected.map(Number)
    return S.sortNotes(S.loadNotes().filter(n => !n.deleted && ids.indexOf(n.id) >= 0), 0)
  },
  render(list, format) {
    const tags = n => n.tags.map(t => this.tagName[t]).filter(Boolean)
    if (format === 'json') {
      return JSON.stringify({
        app: 'notes', v: 1,
        notes: list.map(n => ({ title: n.title, body: n.body, tags: tags(n), color: n.color, pinned: n.pinned, created: n.created, updated: n.updated }))
      })
    }
    if (format === 'md') {
      return list.map(n => '## ' + (n.title || '无标题') + '\n\n' + n.body + (tags(n).length ? '\n\n' + tags(n).map(t => '#' + t).join(' ') : '')).join('\n\n---\n\n')
    }
    return list.map(n => (n.title || '无标题') + '\n' + n.body + '\n（' + S.fmtTime(n.updated) + '）').join('\n\n')
  },
  build() {
    const text = this.render(this.picked(), this.data.format)
    this.text = text
    this.setData({ preview: text.length > PREVIEW_MAX ? text.slice(0, PREVIEW_MAX) + '…' : text, size: text.length })
  },
  // checkbox-group：选中的 value（字符串数组）
  onPick(e) {
    const selected = e.detail.value.slice()
    this.setData({ selected, notes: this.data.notes.map(n => Object.assign({}, n, { on: selected.indexOf(n.id) >= 0 })) })
    this.build()
  },
  onAll() {
    const all = this.data.selected.length === this.data.notes.length
    const selected = all ? [] : this.data.notes.map(n => n.id)
    this.setData({ selected, notes: this.data.notes.map(n => Object.assign({}, n, { on: !all })) })
    this.build()
  },
  // radio-group：选中项的 value 字符串
  onFormat(e) {
    this.setData({ format: e.detail.value })
    this.build()
  },
  onCopy() {
    if (!this.data.selected.length) return wx.showToast({ title: '先勾选要导出的笔记', icon: 'none' })
    wx.setClipboardData({
      data: this.text,
      success: () => wx.showToast({ title: '已复制 ' + this.data.selected.length + ' 篇', icon: 'none' })
    })
  },
  // 导入「备份」格式：追加为新笔记，标签按名字对上（没有的新建）
  onImport() {
    wx.getClipboardData({
      success: res => {
        let notes = null
        try {
          const o = JSON.parse(res.data)
          notes = o && o.app === 'notes' && Array.isArray(o.notes) ? o.notes : null
        } catch (e) {
          notes = null
        }
        if (!notes) {
          wx.showModal({ title: '无法导入', content: '剪贴板里不是「备份」格式。先在本页选「备份（可导入）」并复制。', showCancel: false })
          return
        }
        const ok = notes.filter(n => n && typeof n.body === 'string' && typeof (n.title || '') === 'string' && (String(n.title || '').trim() || n.body.trim()))
        if (!ok.length) return wx.showToast({ title: '没有可导入的笔记', icon: 'none' })
        const tags = S.loadTags()
        const tagId = name => {
          let t = tags.find(x => x.name === name)
          if (!t && name && name.length <= 8) {
            t = { id: 't' + (tags.reduce((m, x) => Math.max(m, Number(x.id.slice(1)) || 0), 0) + 1), name }
            tags.push(t)
          }
          return t ? t.id : ''
        }
        const list = S.loadNotes()
        const now = Date.now()
        ok.forEach(n => {
          list.push({
            id: S.nextId(list), title: String(n.title || '').slice(0, 60), body: n.body.slice(0, 2000),
            tags: (Array.isArray(n.tags) ? n.tags : []).map(tagId).filter(Boolean),
            color: S.COLORS.some(c => c.key === n.color) ? n.color : 'white', pinned: false,
            created: Number(n.created) || now, updated: now, deleted: false
          })
        })
        S.saveTags(tags)
        S.saveNotes(list)
        wx.showToast({ title: '导入 ' + ok.length + ' 篇', icon: 'success' })
        this.onShow()
      },
      fail: () => wx.showToast({ title: '读取剪贴板失败', icon: 'none' })
    })
  }
})
