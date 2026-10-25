const M = require('../../utils/market.js')
const A = require('../../utils/account.js')
Page({
  data: { kw: '', results: [], hot: true },
  onShow() {
    this.refresh()
  },
  refresh() {
    const kw = this.data.kw.trim().toUpperCase()
    const results = M.STOCKS
      .filter(s => !kw || s.code.indexOf(kw) >= 0 || s.name.indexOf(kw) >= 0)
      .map(s => {
        const q = M.quote(s.code)
        return { code: s.code, name: s.name, priceText: q.priceText, pctText: q.pctText, dir: q.dir, watched: A.isWatched(s.code) }
      })
    this.setData({ results, hot: !kw })
  },
  onInput(e) {
    this.setData({ kw: e.detail.value })
    this.refresh()
  },
  onClear() {
    this.setData({ kw: '' })
    this.refresh()
  },
  // 「＋」是 catchtap：加自选不进详情
  onAdd(e) {
    // 微信里 data-code="{{item.code}}" 保留字符串类型；String() 兜住把 '600519' 还原成数字的实现
    const code = String(e.currentTarget.dataset.code)
    if (A.addWatch(code)) wx.showToast({ title: '已加入自选', icon: 'success' })
    else wx.showToast({ title: '已在自选中', icon: 'none' })
    this.refresh()
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?code=' + e.currentTarget.dataset.code })
  }
})
