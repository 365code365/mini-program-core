const A = require('../../utils/account.js')
const M = require('../../utils/market.js')
function money(x) {
  return (x > 0 ? '+' : '') + x.toFixed(2)
}
Page({
  data: { s: null, rows: [], mvText: '', pnlText: '', cashText: '', totalText: '', tickNo: 0 },
  onShow() {
    this.refresh()
  },
  // 行情每跳一次：市值与浮动盈亏重算
  onTick() {
    this.refresh()
  },
  refresh() {
    const s = A.summary()
    const rows = s.rows.map(r => Object.assign({}, r, { pnlText: money(r.pnl), pnlPctText: money(r.pnlPct) + '%', mvText: r.mv.toFixed(2) }))
    this.setData({
      s: { mv: s.mv, pnl: s.pnl, cash: s.cash, total: s.total, dir: s.dir },
      rows,
      mvText: s.mv.toFixed(2),
      pnlText: money(s.pnl),
      cashText: s.cash.toFixed(2),
      totalText: s.total.toFixed(2),
      tickNo: M.ticks()
    })
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?code=' + e.currentTarget.dataset.code })
  },
  onSell(e) {
    wx.navigateTo({ url: '/pages/trade/trade?side=sell&code=' + e.currentTarget.dataset.code })
  },
  goOrders() {
    wx.navigateTo({ url: '/pages/orders/orders' })
  }
})
