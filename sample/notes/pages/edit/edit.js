const S = require('../../utils/store.js')
// 输入停下 500ms 后才落盘：每个字都写 storage 太频繁
const DEBOUNCE = 500
Page({
  data: {
    isNew: true,
    form: null,
    tagOpts: [],
    colors: S.COLORS,
    bg: '#ffffff',
    words: 0,
    saveState: '',
    saveCount: 0
  },
  onLoad(query) {
    const list = S.loadNotes()
    const id = Number(query.id)
    const n = list.find(i => i.id === id && !i.deleted)
    const now = Date.now()
    const form = n ? Object.assign({}, n) : {
      id: S.nextId(list), title: '', body: '', tags: [], color: 'white', pinned: false, created: now, updated: now, deleted: false
    }
    this.setData({ isNew: !n, form })
    wx.setNavigationBarTitle({ title: n ? '编辑笔记' : '新建笔记' })
    this.syncDerived()
  },
  onUnload() {
    // 返回时还有没落盘的改动：立刻保存，不丢字
    if (this._timer) {
      clearTimeout(this._timer)
      this._timer = null
      this.save()
    }
  },
  syncDerived() {
    const f = this.data.form
    const tagOpts = S.loadTags().map(t => ({ id: t.id, name: t.name, on: f.tags.indexOf(t.id) >= 0 }))
    this.setData({ tagOpts, bg: S.colorHex(f.color), words: S.wordCount(f.body) })
  },
  schedule() {
    if (this._timer) clearTimeout(this._timer)
    this.setData({ saveState: '编辑中…' })
    this._timer = setTimeout(() => {
      this._timer = null
      this.save()
    }, DEBOUNCE)
  },
  save() {
    const f = this.data.form
    // 新笔记标题正文都空着：不建一条空笔记
    if (this.data.isNew && !f.title.trim() && !f.body.trim()) {
      this.setData({ saveState: '' })
      return
    }
    const list = S.loadNotes()
    const k = list.findIndex(i => i.id === f.id)
    const rec = Object.assign({}, f, { updated: Date.now() })
    if (k >= 0) list[k] = rec
    else list.push(rec)
    S.saveNotes(list)
    this.setData({ isNew: false, saveState: '已保存', saveCount: this.data.saveCount + 1, 'form.updated': rec.updated })
  },
  onTitle(e) {
    this.setData({ 'form.title': e.detail.value })
    this.schedule()
  },
  onBody(e) {
    const body = e.detail.value
    this.setData({ 'form.body': body, words: S.wordCount(body) })
    this.schedule()
  },
  // checkbox-group：e.detail.value 是选中的 value 数组
  onTags(e) {
    this.setData({ 'form.tags': e.detail.value.slice() })
    this.syncDerived()
    this.schedule()
  },
  // radio-group：e.detail.value 是选中那一项的 value 字符串
  onColor(e) {
    this.setData({ 'form.color': e.detail.value })
    this.syncDerived()
    this.schedule()
  },
  onTrash() {
    if (this._timer) {
      clearTimeout(this._timer)
      this._timer = null
    }
    const list = S.loadNotes()
    const n = list.find(i => i.id === this.data.form.id)
    if (n) {
      n.deleted = true
      n.pinned = false
      S.saveNotes(list)
    }
    wx.showToast({ title: '已移到回收站', icon: 'none' })
    wx.navigateBack()
  }
})
