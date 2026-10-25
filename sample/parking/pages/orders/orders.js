const { lotById } = require('../../utils/data.js')
const F = require('../../utils/fee.js')
const S = require('../../utils/store.js')

const TABS = ['进行中', '已完成']

Page({
  data: { tabs: TABS, tab: 0, list: [], counts: [0, 0] },
  onLoad(query) {
    this.setData({ tab: Number(query.tab) === 1 ? 1 : 0 })
  },
  onShow() {
    this.refresh()
  },
  refresh() {
    // 进行中 = 在场的车（待缴费）+ 未结束的预约；已完成 = 已支付 / 已取消
    const parking = S.sessions().map(s => ({
      id: 'S-' + s.plate, kind: 'parking', plate: s.plate, lotName: lotById(s.lotId).name,
      desc: s.inTime + ' 入场', statusText: '停车中', amountText: ''
    }))
    const orders = S.orders().map(o => Object.assign({}, o, { kind: o.type, amountText: F.money(o.amount) }))
    const ongoing = parking.concat(orders.filter(o => o.status === 'ongoing'))
    const done = orders.filter(o => o.status !== 'ongoing')
    this.setData({ list: this.data.tab === 0 ? ongoing : done, counts: [ongoing.length, done.length] })
  },
  onTab(e) {
    this.setData({ tab: Number(e.currentTarget.dataset.i) })
    this.refresh()
  },
  goPay(e) {
    wx.navigateTo({ url: '/pages/pay/pay?plate=' + encodeURIComponent(e.currentTarget.dataset.plate) })
  },
  cancel(e) {
    const id = e.currentTarget.dataset.id
    wx.showModal({
      title: '取消预约',
      content: '取消后车位将释放给其他车主，预约费用原路退回',
      confirmText: '取消预约',
      cancelText: '再想想',
      success: r => {
        if (!r.confirm) return
        const list = S.orders()
        const o = list.find(i => i.id === id)
        o.status = 'canceled'
        o.statusText = '已取消'
        S.saveOrders(list)
        S.addFree(o.lotId, 1)
        this.refresh()
      }
    })
  }
})
