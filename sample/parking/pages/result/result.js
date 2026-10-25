const S = require('../../utils/store.js')
const F = require('../../utils/fee.js')

Page({
  data: { order: null, amountText: '', missing: false },
  onLoad(query) {
    const order = S.orders().find(o => o.id === query.id)
    if (!order) return this.setData({ missing: true })
    this.setData({ order, amountText: F.money(order.amount), feeText: F.money(order.fee || order.amount), offText: order.off ? '-' + F.money(order.off) : '' })
  },
  goOrders() {
    wx.redirectTo({ url: '/pages/orders/orders?tab=1' })
  },
  goHome() {
    wx.reLaunch({ url: '/pages/index/index' })
  }
})
