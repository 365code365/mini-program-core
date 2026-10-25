const S = require('../../utils/store.js')
Page({
  data: {
    passengers: [],
    orderCount: 0,
    spent: 0
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 2 })
    this.refresh()
  },
  refresh() {
    const orders = S.orders().filter(o => o.status !== 'cancelled')
    this.setData({
      passengers: S.passengers().map(p => Object.assign({}, p, { idText: S.maskId(p.idcard) })),
      orderCount: orders.length,
      spent: orders.reduce((m, o) => m + o.total, 0)
    })
  },
  onAdd() {
    wx.navigateTo({ url: '/pages/passenger/passenger' })
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
  }
})
