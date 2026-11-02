const S = require('../../utils/store.js')
Page({
  data: {
    card: null,
    colName: '',
    prioNames: S.PRIOS.map(p => p.name),
    ownerNames: S.MEMBERS.map(m => m.name),
    ownerIdx: 0,
    prioColor: '',
    overdue: false,
    doneIds: [],
    pct: 0,
    progressText: '',
    subDraft: '',
    raw: {},
    comments: [],
    commentDraft: ''
  },
  onLoad(query) {
    this.id = Number(query.id)
    this.reload()
  },
  reload() {
    const c = S.find(this.id)
    if (!c) {
      this.setData({ card: null })
      return
    }
    const p = S.progress(c)
    const col = S.COLS.find(i => i.key === c.col)
    this.setData({
      card: c,
      colName: col ? col.name : '',
      ownerIdx: Math.max(0, S.MEMBERS.findIndex(m => m.id === c.owner)),
      prioColor: S.PRIOS[c.prio].color,
      overdue: S.isOverdue(c),
      pct: p.pct,
      progressText: p.done + '/' + p.total,
      // 评论：最新的在上面
      comments: (c.comments || []).slice().reverse().map(m => {
        const who = S.member(m.who)
        return { id: m.id, text: m.text, name: who.name, color: who.color, initial: who.name.slice(-1), time: S.timeText(m.at) }
      })
    })
  },
  patch(fn) {
    S.update(this.id, fn)
    this.reload()
  },
  onTitle(e) {
    const v = e.detail.value
    this.patch(c => { c.title = v })
  },
  onTitleFocus() {
    this._title0 = this.data.card.title
  },
  // 标题不能清空：失焦时是空的就恢复成改之前的
  onTitleBlur() {
    const t = (this.data.card.title || '').trim()
    if (!t) {
      const old = this._title0 || '未命名卡片'
      this.patch(c => { c.title = old })
      wx.showToast({ title: '标题不能为空，已恢复', icon: 'none' })
    } else if (this._title0 !== undefined && t !== this._title0) {
      S.log('edit', this.data.card, '把标题改为「' + t + '」')
    }
    this._title0 = undefined
  },
  onDesc(e) {
    const v = e.detail.value
    this.patch(c => { c.desc = v })
  },
  // picker selector：e.detail.value 是下标（number）
  onPrio(e) {
    const v = e.detail.value
    this.setData({ 'raw.prio': v })
    this.patch(c => { c.prio = Number(v) })
    S.log('edit', this.data.card, '把优先级改为「' + S.PRIOS[Number(v)].name + '」')
  },
  onOwner(e) {
    const v = e.detail.value
    this.setData({ 'raw.owner': v })
    this.patch(c => { c.owner = S.MEMBERS[Number(v)].id })
    S.log('edit', this.data.card, '把负责人改为' + S.MEMBERS[Number(v)].name)
  },
  // picker date：e.detail.value 是 'YYYY-MM-DD' 字符串
  onDue(e) {
    const v = e.detail.value
    this.setData({ 'raw.due': v })
    this.patch(c => { c.due = v })
    S.log('edit', this.data.card, '把截止日期改为 ' + v)
  },
  // checkbox-group：选中的子任务 id（字符串数组）
  onSubs(e) {
    const ids = e.detail.value.map(Number)
    this.patch(c => { c.subs.forEach(s => { s.done = ids.indexOf(s.id) >= 0 }) })
    const p = S.progress(S.find(this.id))
    if (p.total && p.done === p.total) {
      S.log('edit', this.data.card, '完成了全部 ' + p.total + ' 个子任务')
      wx.showToast({ title: '子任务全部完成', icon: 'success' })
    }
  },
  onSubDraft(e) {
    this.setData({ subDraft: e.detail.value })
  },
  onSubAdd() {
    const text = this.data.subDraft.trim()
    if (!text) {
      wx.showToast({ title: '写点子任务内容', icon: 'none' })
      return
    }
    this.patch(c => { c.subs.push({ id: c.subs.reduce((m, s) => Math.max(m, s.id), 0) + 1, text, done: false }) })
    this.setData({ subDraft: '' })
  },
  onMove() {
    const c = this.data.card
    wx.showActionSheet({
      itemList: S.COLS.map(i => (i.key === c.col ? i.name + '（当前）' : '移到「' + i.name + '」')),
      success: res => {
        const to = S.COLS[res.tapIndex].key
        if (to === c.col) return
        this.patch(x => { x.col = to })
        S.log('move', c, '从「' + S.colName(c.col) + '」移到「' + S.colName(to) + '」')
        wx.showToast({ title: '已移到' + S.COLS[res.tapIndex].name, icon: 'none' })
      }
    })
  },
  onBackBoard() {
    wx.reLaunch({ url: '/pages/board/board' })
  },
  onCommentDraft(e) {
    this.setData({ commentDraft: e.detail.value })
  },
  // 发评论：去掉首尾空白，1~200 字
  onComment() {
    const text = this.data.commentDraft.trim()
    if (!text) return wx.showToast({ title: '写点评论内容', icon: 'none' })
    if (text.length > 200) return wx.showToast({ title: '评论最多 200 字', icon: 'none' })
    this.patch(c => {
      c.comments = c.comments || []
      c.comments.push({ id: c.comments.reduce((m, i) => Math.max(m, i.id), 0) + 1, who: S.ME, text, at: Date.now() })
    })
    S.log('comment', this.data.card, '评论：' + (text.length > 20 ? text.slice(0, 20) + '…' : text))
    this.setData({ commentDraft: '' })
  },
  // 只能删自己的评论
  onCommentLong(e) {
    const id = Number(e.currentTarget.dataset.id)
    wx.showActionSheet({
      itemList: ['复制评论', '删除评论'],
      success: res => {
        const m = (this.data.card.comments || []).find(i => i.id === id)
        if (!m) return
        if (res.tapIndex === 0) wx.setClipboardData({ data: m.text })
        else this.patch(c => { c.comments = c.comments.filter(i => i.id !== id) })
      }
    })
  },
  onDelete() {
    wx.showModal({
      title: '删除卡片',
      content: '「' + this.data.card.title + '」删除后无法恢复',
      confirmText: '删除',
      confirmColor: '#f5222d',
      success: res => {
        if (!res.confirm) return
        S.save(S.load().filter(c => c.id !== this.id))
        S.log('delete', this.data.card, '删除了卡片')
        wx.navigateBack()
      }
    })
  }
})
