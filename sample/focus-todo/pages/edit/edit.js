const S = require('../../utils/store.js')

Page({
  data: {
    isNew: false,
    form: null,
    prios: S.PRIORITY,
    prioColor: '#c4c9d2',
    dateLabel: '',
    noteLen: 0,
    repeats: ['不重复', '每天', '工作日', '每周', '每月']
  },

  onLoad(query) {
    const id = Number(query.id)
    const it = S.loadTodos().find(i => i.id === id)
    const form = it ? Object.assign({ repeat: 0 }, it) : {
      id: 0, title: '', priority: 0, date: S.today(), time: '', remind: false, note: '', done: false, pomos: 0, repeat: 0
    }
    this.setData({ isNew: !it, form })
    wx.setNavigationBarTitle({ title: it ? '编辑待办' : '新建待办' })
    this.sync()
  },

  sync() {
    const f = this.data.form
    this.setData({
      prioColor: S.PRIORITY[f.priority].color,
      dateLabel: S.label(f.date) + (f.date === S.today() || f.date === S.dayOffset(1) ? '  ' + f.date.slice(5) : ''),
      noteLen: (f.note || '').length
    })
  },

  onTitle(e) { this.setData({ 'form.title': e.detail.value }) },
  onPrio(e) { this.setData({ 'form.priority': Number(e.currentTarget.dataset.i) }); this.sync() },
  onDate(e) { this.setData({ 'form.date': e.detail.value }); this.sync() },
  onTime(e) { this.setData({ 'form.time': e.detail.value }) },
  onRepeat(e) { this.setData({ 'form.repeat': Number(e.detail.value) }) },
  onNote(e) { this.setData({ 'form.note': e.detail.value, noteLen: e.detail.value.length }) },
  onToggleDone() { this.setData({ 'form.done': !this.data.form.done }) },

  onRemind(e) {
    const on = e.detail.value
    this.setData({ 'form.remind': on })
    if (on && !this.data.form.time) {
      this.setData({ 'form.time': '09:00' })
      wx.showToast({ title: '默认 09:00 提醒', icon: 'none' })
    }
  },

  onFocus() {
    getApp().globalData.focusTaskId = this.data.form.id
    wx.switchTab({ url: '/pages/timer/timer' })
  },

  onSave() {
    const f = this.data.form
    if (!f.title || !f.title.trim()) {
      wx.showToast({ title: '先写下要做的事', icon: 'none' })
      return
    }
    const list = S.loadTodos()
    if (this.data.isNew) {
      f.id = list.reduce((m, i) => Math.max(m, i.id), 0) + 1
      list.push(f)
    } else {
      const k = list.findIndex(i => i.id === f.id)
      list[k] = f
    }
    S.saveTodos(list)
    wx.showToast({ title: '已保存', icon: 'success' })
    setTimeout(() => wx.navigateBack(), 500)
  },

  onDelete() {
    wx.showModal({
      title: '删除待办',
      content: '「' + this.data.form.title + '」删除后无法恢复',
      confirmText: '删除',
      confirmColor: '#f2453d',
      success: res => {
        if (!res.confirm) return
        S.saveTodos(S.loadTodos().filter(i => i.id !== this.data.form.id))
        wx.navigateBack()
      }
    })
  }
})
