// 订单详情：缴费单（费用、券、开票）/ 预约单（导航去停车场、取消预约、再次预约）
const { lotById } = require('../../utils/data.js')
const F = require('../../utils/fee.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
Page({
  data: { o: null, lot: null, invoiced: false },
  goOrders() { wx.redirectTo({ url: '/pages/orders/orders' }) },
  onLoad(q) {
    this.id = String(q.id || '')
  },
  onShow() {
    const o = S.orders().find(x => x.id === this.id)
    if (!o) return this.setData({ o: null })
    const lot = lotById(o.lotId)
    this.setData({
      lot: { id: lot.id, name: lot.name, addr: lot.addr, phone: lot.phone },
      invoiced: S.invoiced().indexOf(o.id) >= 0,
      o: Object.assign({}, o, {
        amountText: F.money(o.amount), feeText: F.money(o.fee === undefined ? o.amount : o.fee),
        offText: o.off ? '-' + F.money(o.off) : ''
      })
    })
  },
  onLot() { wx.navigateTo({ url: '/pages/lot/lot?id=' + this.data.lot.id }) },
  onNav() { host.openMap(lotById(this.data.lot.id)) },
  onCall() { host.call(this.data.lot.phone) },
  onCopy() { host.copy(this.data.o.id) },
  onInvoice() {
    wx.navigateTo({ url: this.data.invoiced ? '/pages/invoice/invoice?tab=list' : '/pages/invoice/invoice?ids=' + this.data.o.id })
  },
  onCancel() {
    wx.showModal({
      title: '取消预约',
      content: '取消后车位将释放给其他车主（预约不预收费用）',
      confirmText: '取消预约',
      cancelText: '再想想',
      success: r => {
        if (!r.confirm) return
        const list = S.orders()
        const o = list.find(i => i.id === this.id)
        if (!o || o.status !== 'ongoing') return
        o.status = 'canceled'
        o.statusText = '已取消'
        S.saveOrders(list)
        S.addFree(o.lotId, 1)
        this.onShow()
        wx.showToast({ title: '已取消预约', icon: 'none' })
      }
    })
  }
})
