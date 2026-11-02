// 寄件记录：按状态筛选，取消（待揽收）/ 查看物流 / 再寄一单
const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const TABS = [
  { key: 'all', text: '全部' },
  { key: 'pending', text: '待揽收' },
  { key: 'going', text: '运输中' },
  { key: 'signed', text: '已签收' },
  { key: 'cancelled', text: '已取消' }
]
function match(tab, k) {
  if (tab === 'all') return true
  if (tab === 'going') return k === 'transit' || k === 'deliver'
  return tab === k
}
Page({
  data: { tabs: TABS, tab: 'all', list: [], total: 0 },
  onLoad(q) {
    if (TABS.some(t => t.key === q.tab)) this.setData({ tab: q.tab })
  },
  onShow() { this.refresh() },
  refresh() {
    const all = S.getOrders().map(o => {
      const st = S.stageOf(o)
      const c = D.CARRIERS[o.carrier]
      return {
        id: o.id, no: o.no, productName: o.productName, short: c.short, color: c.color,
        st: st.key, stText: st.text, canCancel: st.canCancel,
        from: o.fromCity, to: o.toCity, sender: o.sender.name, receiver: o.receiver.name,
        goods: o.goods, weight: o.weight, payText: o.pay.toFixed(2), time: S.stamp(o.createdAt)
      }
    })
    this.setData({ total: all.length, list: all.filter(o => match(this.data.tab, o.st)) })
  },
  onTab(e) {
    this.setData({ tab: String(e.currentTarget.dataset.k) })
    this.refresh()
  },
  onOpen(e) { wx.navigateTo({ url: '/pages/order/order?id=' + e.currentTarget.dataset.id }) },
  onTrack(e) { wx.navigateTo({ url: '/pages/detail/detail?no=' + e.currentTarget.dataset.no }) },
  onCancel(e) {
    const id = String(e.currentTarget.dataset.id)
    const o = S.findOrder(id)
    wx.showModal({
      title: '取消寄件',
      content: '快递员还未上门，取消后运费 ¥' + o.pay.toFixed(2) + ' 原路退回' + (o.couponTitle ? '，「' + o.couponTitle + '」退回账户' : ''),
      confirmText: '取消寄件',
      cancelText: '再想想',
      confirmColor: '#f2453d',
      success: r => {
        if (!r.confirm) return
        if (!S.cancelOrder(id)) { wx.showToast({ title: '快递员已揽收，不能取消', icon: 'none' }); this.refresh(); return }
        this.refresh()
        wx.showToast({ title: '已取消，运费原路退回', icon: 'none' })
      }
    })
  },
  onAgain(e) {
    const o = S.findOrder(String(e.currentTarget.dataset.id))
    getApp().globalData.resend = { sender: o.sender, receiver: o.receiver, goods: o.goods, weight: o.weight, product: o.carrier, insured: o.insured }
    wx.switchTab({ url: '/pages/send/send' })
  },
  goSend() { wx.switchTab({ url: '/pages/send/send' }) }
})
