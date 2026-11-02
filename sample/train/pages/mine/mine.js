const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
Page({
  data: {
    passengers: [],
    orderCount: 0,
    spent: 0,
    todo: 0
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 2 })
    this.refresh()
  },
  refresh() {
    S.sync()
    const orders = S.orders().filter(o => o.status !== 'cancelled')
    this.setData({
      passengers: S.passengers().map(p => Object.assign({}, p, { idText: S.maskId(p.idcard), typeText: p.type && p.type !== '成人票' ? p.type : '' })),
      orderCount: orders.length,
      spent: orders.reduce((m, o) => m + o.total, 0),
      todo: S.orders().filter(o => !S.orderState(o).done).length
    })
  },
  onOrders() {
    wx.switchTab({ url: '/pages/orders/orders' })
  },
  onAdd() {
    wx.navigateTo({ url: '/pages/passenger/passenger' })
  },
  onEdit(e) {
    wx.navigateTo({ url: '/pages/passenger/passenger?id=' + e.currentTarget.dataset.id })
  },
  onRemove(e) {
    const p = this.data.passengers[Number(e.currentTarget.dataset.index)]
    if (p.self) return wx.showToast({ title: '本人信息不能删除', icon: 'none' })
    wx.showModal({
      title: '删除乘车人',
      content: '确定删除乘车人「' + p.name + '」吗？',
      success: res => {
        if (!res.confirm) return
        S.removePassenger(p.id)
        this.refresh()
      }
    })
  },
  onCall() {
    host.call('12306')
  },
  onRule() {
    wx.showModal({
      title: '退改签规则',
      content: '退票：开车前 8 天以上免费；48 小时~8 天收 5%；24~48 小时收 10%；不足 24 小时收 20%，每张最低 2 元。\n改签：开车前可在线改签一次，新票价高补差价、低退差价。\n候补：无票时可预付候补，未兑现全额退款。',
      showCancel: false,
      confirmText: '知道了'
    })
  },
  onAbout() {
    wx.showModal({
      title: '畅行火车票 v1.1',
      content: '查车次、看余票、在线选座、候补购票与退改签。乘车人与订单只保存在本机。',
      showCancel: false,
      confirmText: '知道了'
    })
  }
})
