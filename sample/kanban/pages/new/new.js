const S = require('../../utils/store.js')
Page({
  data: {
    colNames: S.COLS.map(c => c.name),
    prioNames: S.PRIOS.map(p => p.name),
    ownerNames: S.MEMBERS.map(m => m.name),
    form: { title: '', desc: '', col: 0, prio: 1, owner: 0, due: '' },
    error: '',
    titleLen: 0
  },
  onLoad(query) {
    const col = Math.max(0, S.COLS.findIndex(c => c.key === query.col))
    this.setData({ 'form.col': col, 'form.due': S.dayOffset(3) })
  },
  onTitle(e) {
    const v = e.detail.value
    this.setData({ 'form.title': v, titleLen: v.length, error: '' })
  },
  onDesc(e) { this.setData({ 'form.desc': e.detail.value }) },
  onCol(e) { this.setData({ 'form.col': Number(e.detail.value) }) },
  onPrio(e) { this.setData({ 'form.prio': Number(e.detail.value) }) },
  onOwner(e) { this.setData({ 'form.owner': Number(e.detail.value) }) },
  onDue(e) { this.setData({ 'form.due': e.detail.value }) },
  // 校验：标题必填（去掉首尾空白）、最多 30 字
  validate() {
    const t = this.data.form.title.trim()
    if (!t) return '标题不能为空'
    if (t.length > 30) return '标题最多 30 个字'
    return ''
  },
  onSubmit() {
    const error = this.validate()
    if (error) {
      this.setData({ error })
      wx.showToast({ title: error, icon: 'none' })
      return
    }
    const f = this.data.form
    const list = S.load()
    const card = {
      id: S.nextId(list), title: f.title.trim(), desc: f.desc, col: S.COLS[f.col].key,
      prio: f.prio, owner: S.MEMBERS[f.owner].id, due: f.due, subs: []
    }
    list.push(card)
    S.save(list)
    wx.showToast({ title: '已创建', icon: 'success' })
    wx.navigateBack()
  }
})
