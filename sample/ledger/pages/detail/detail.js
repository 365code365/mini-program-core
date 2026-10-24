const store = require('../../utils/store.js')

Page({
  data: { id: '', rec: null, missing: false, typeText: '', dateText: '', sameCatText: '0.00', sameCatCount: 0 },
  onLoad(q) { this.setData({ id: (q && q.id) || '' }) },
  onShow() {
    let r = store.get(this.data.id)
    // 没带 id（直接打开详情页）时展示最近一笔，方便预览
    if (!r && !this.data.id) {
      const list = store.all().slice().sort((a, b) => (b.date > a.date ? 1 : b.date < a.date ? -1 : b.ts - a.ts))
      r = list[0] || null
      if (r) this.setData({ id: r.id })
    }
    if (!r) { this.setData({ missing: true, rec: null }); return }
    const d = store.parseDate(r.date)
    const same = store.monthRecords(d.getFullYear(), d.getMonth() + 1).filter(x => x.type === r.type && x.cat === r.cat)
    this.setData({
      rec: store.decorate(r),
      missing: false,
      typeText: r.type === 'out' ? '支出' : '收入',
      dateText: d.getFullYear() + '年' + (d.getMonth() + 1) + '月' + d.getDate() + '日 ' + store.WEEK[d.getDay()],
      sameCatText: store.money(store.sum(same, r.type)),
      sameCatCount: same.length
    })
  },
  onEdit() { wx.navigateTo({ url: '/pages/add/add?id=' + this.data.id }) },
  onDelete() {
    const r = this.data.rec
    wx.showModal({
      title: '删除这笔账单？',
      content: r.cat + ' ' + r.amountText + ' 删除后不可恢复',
      confirmText: '删除',
      confirmColor: '#f04848',
      success: (res) => {
        if (!res.confirm) return
        store.remove(this.data.id)
        wx.showToast({ title: '已删除', icon: 'success' })
        setTimeout(() => wx.navigateBack(), 600)
      }
    })
  }
})
