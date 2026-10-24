const W = require('../../utils/data.js')

Page({
  data: { city: null, list: [], focus: '', condName: '', hi: 0, lo: 0 },
  onLoad(q) {
    const c = W.byId((q && q.id) || W.current())
    const u = W.unit()
    wx.setNavigationBarTitle({ title: c.name + ' · 生活指数' })
    this.setData({ city: c, list: W.lifeIndex(c), focus: (q && q.key) || '', condName: W.COND[c.cond].name, hi: W.fmtT(c.hi, u), lo: W.fmtT(c.lo, u), bg: W.COND[c.cond].bg })
  },
  onTap(e) {
    const key = e.currentTarget.dataset.key
    this.setData({ focus: this.data.focus === key ? '' : key })
  },
  onShareTip() {
    const f = this.data.list.filter(x => x.key === this.data.focus)[0] || this.data.list[0]
    wx.showModal({ title: f.name + '指数：' + f.level, content: f.desc, showCancel: false, confirmText: '知道了' })
  }
})
