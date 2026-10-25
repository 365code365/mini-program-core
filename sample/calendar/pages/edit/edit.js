const D = require('../../utils/date.js')
const S = require('../../utils/store.js')

function toMin(hhmm) {
  const p = hhmm.split(':')
  return Number(p[0]) * 60 + Number(p[1])
}
// 开始时间往后推一小时作为默认结束（不跨天）
function plusHour(hhmm) {
  const m = Math.min(toMin(hhmm) + 60, 23 * 60 + 59)
  return D.pad(Math.floor(m / 60)) + ':' + D.pad(m % 60)
}

Page({
  data: {
    isNew: true,
    form: null,
    dateLabel: '',
    repeats: S.REPEATS,
    reminds: S.REMINDS,
    colors: S.COLORS,
    err: ''
  },
  onLoad(query) {
    const ev = query.id ? S.getEvent(query.id) : null
    const settings = S.loadSettings()
    const form = ev ? Object.assign({}, ev) : {
      id: 0, title: '', allDay: false, start: '09:00', end: '10:00',
      date: query.date || S.today(), repeat: 0, remind: settings.defaultRemind, color: 1, note: ''
    }
    this.setData({ isNew: !ev, form, dateLabel: D.longLabel(form.date) })
    wx.setNavigationBarTitle({ title: ev ? '编辑日程' : '新建日程' })
  },
  fail(msg) {
    this.setData({ err: msg })
    wx.showToast({ title: msg, icon: 'none' })
  },
  onTitle(e) { this.setData({ 'form.title': e.detail.value, err: '' }) },
  onNote(e) { this.setData({ 'form.note': e.detail.value }) },
  onAllDay(e) { this.setData({ 'form.allDay': e.detail.value, err: '' }) },
  onStart(e) {
    const start = e.detail.value
    const patch = { 'form.start': start, err: '' }
    // 开始挪到结束之后：结束跟着顺延一小时
    if (toMin(this.data.form.end) < toMin(start)) patch['form.end'] = plusHour(start)
    this.setData(patch)
  },
  onEnd(e) {
    const end = e.detail.value
    if (toMin(end) < toMin(this.data.form.start)) {
      // 不接受这次选择，保留原来的结束时间
      this.fail('结束时间不能早于开始时间')
      return
    }
    this.setData({ 'form.end': end, err: '' })
  },
  onDate(e) {
    this.setData({ 'form.date': e.detail.value, dateLabel: D.longLabel(e.detail.value) })
  },
  onRepeat(e) { this.setData({ 'form.repeat': Number(e.detail.value) }) },
  onRemind(e) { this.setData({ 'form.remind': Number(e.detail.value) }) },
  onColor(e) { this.setData({ 'form.color': Number(e.detail.value) }) },
  onSave() {
    const f = Object.assign({}, this.data.form)
    f.title = (f.title || '').trim()
    if (!f.title) return this.fail('请填写日程标题')
    if (f.title.length > 30) return this.fail('标题最多 30 个字')
    if (!f.allDay && toMin(f.end) < toMin(f.start)) return this.fail('结束时间不能早于开始时间')
    const list = S.loadEvents()
    if (this.data.isNew) {
      f.id = list.reduce((m, i) => Math.max(m, i.id), 0) + 1
      list.push(f)
    } else {
      const k = list.findIndex(i => i.id === f.id)
      if (k < 0) list.push(f)
      else list[k] = f
    }
    S.saveEvents(list)
    getApp().globalData.lastSaved = f.id
    wx.showToast({ title: '已保存', icon: 'success' })
    // 直接打开编辑页（没有上一页）时回月视图
    if (getCurrentPages().length > 1) wx.navigateBack()
    else wx.reLaunch({ url: '/pages/month/month' })
  }
})
