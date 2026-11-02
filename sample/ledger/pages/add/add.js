const store = require('../../utils/store.js')

const KEYS = ['1', '2', '3', 'del', '4', '5', '6', 'clear', '7', '8', '9', 'again', '.', '0', '00', 'ok']

Page({
  data: {
    editId: '',
    formSeq: 0,
    type: 'out',
    cats: [],
    cat: '餐饮',
    amount: '0',
    note: '',
    date: '',
    dateText: '',
    keys: KEYS.map(k => ({ k: k, label: k === 'del' ? '删除' : k === 'clear' ? '清零' : k === 'again' ? '再记' : k === 'ok' ? '保存' : k }))
  },
  onLoad(q) {
    const today = store.today()
    if (q && q.id) {
      const r = store.get(q.id)
      if (r) {
        wx.setNavigationBarTitle({ title: '编辑账单' })
        this.setData({ editId: r.id, type: r.type, cat: r.cat, amount: String(r.amount), note: r.note || '', date: r.date, dateText: this.fmt(r.date) })
        return
      }
    }
    this.setData({ date: today, dateText: this.fmt(today) })
  },
  // 分类管理页回来：重读分类；选中的分类被删了就退回第一个
  onShow() {
    const cats = store.cats(this.data.type)
    const keep = cats.some(c => c.name === this.data.cat) || (this.data.editId && this.data.cat)
    this.setData({ cats: cats, cat: keep ? this.data.cat : cats[0].name })
  },
  onManage() { wx.navigateTo({ url: '/pages/cats/cats?type=' + this.data.type }) },
  fmt(ds) {
    const d = store.parseDate(ds)
    const label = ds === store.today() ? '今天' : store.WEEK[d.getDay()]
    return (d.getMonth() + 1) + '月' + d.getDate() + '日 ' + label
  },
  onType(e) {
    const t = e.currentTarget.dataset.t
    if (t === this.data.type) return
    const cats = store.cats(t)
    this.setData({ type: t, cats: cats, cat: cats[0].name })
  },
  onCat(e) { this.setData({ cat: e.currentTarget.dataset.name }) },
  onNote(e) { this.setData({ note: e.detail.value }) },
  onDate(e) { this.setData({ date: e.detail.value, dateText: this.fmt(e.detail.value) }) },
  onKey(e) {
    // String()：引擎把看起来像数字的 dataset 值转成了 number（微信里是字符串）
    const k = String(e.currentTarget.dataset.k)
    let a = this.data.amount
    if (k === 'ok') return this.save(false)
    if (k === 'again') return this.save(true)
    if (k === 'del') a = a.length > 1 ? a.slice(0, -1) : '0'
    else if (k === 'clear') a = '0'
    else if (k === '.') { if (a.indexOf('.') < 0) a += '.' }
    else {
      const dot = a.indexOf('.')
      if (dot >= 0 && a.length - dot > 2) return              // 最多两位小数
      if (dot < 0 && a.replace(/^0/, '').length >= 8) return  // 最多 8 位整数
      if (a === '0') a = k === '00' ? '0' : k
      else a += k
      if (dot >= 0 && a.length - dot > 3) a = a.slice(0, dot + 3)
    }
    this.setData({ amount: a })
  },
  save(again) {
    const v = parseFloat(this.data.amount)
    if (!(v > 0)) { wx.showToast({ title: '请输入金额', icon: 'none' }); return }
    const before = store.monthUsage()
    store.upsert({ id: this.data.editId || undefined, type: this.data.type, cat: this.data.cat, amount: Math.round(v * 100) / 100, note: this.data.note.trim(), date: this.data.date })
    const after = store.monthUsage()
    // 这一笔让本月支出跨过 80% / 100%：弹预算提醒（预算页的「超支提醒」关掉就只在跨 100% 时提醒）
    const remind = wx.getStorageSync('ledger_remind') !== false
    let warn = ''
    if (before < 1 && after >= 1) warn = '本月支出已超出预算 ¥' + store.money((after - 1) * store.getBudget())
    else if (remind && before < 0.8 && after >= 0.8) warn = '本月预算已用 ' + Math.round(after * 100) + '%，剩余 ¥' + store.money((1 - after) * store.getBudget())
    if (again) {
      this.setData({ amount: '0', note: '', formSeq: (this.data.formSeq || 0) + 1 })
      if (warn) wx.showModal({ title: '预算提醒', content: warn, showCancel: false, confirmText: '知道了' })
      else wx.showToast({ title: '已记下，继续', icon: 'success' })
      return
    }
    if (warn) {
      wx.showModal({ title: '预算提醒', content: warn, showCancel: false, confirmText: '知道了', complete: () => wx.navigateBack() })
      return
    }
    wx.showToast({ title: this.data.editId ? '已修改' : '记账成功', icon: 'success' })
    setTimeout(() => wx.navigateBack(), 600)
  }
})
