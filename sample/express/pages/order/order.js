// 寄件订单详情：进度、寄 / 收件人、物品、费用（运费 / 保价 / 券 / 实付）、取消退款、查看物流、联系快递员、再寄一单
const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const STEPS = ['已下单', '已揽收', '派送中', '已签收']
const TIPS = {
  pending: '快递员将按预约时间上门，上门前可免费取消',
  transit: '快递员已揽收，包裹正在运输中',
  deliver: '包裹正在派送，请保持电话畅通',
  signed: '包裹已签收，感谢使用',
  cancelled: '订单已取消，运费与优惠券已原路退回'
}
Page({
  data: { o: null, st: null, steps: [], tip: '' },
  onLoad(q) { this.id = String(q.id || '') },
  onShow() { this.load() },
  load() {
    const o = S.findOrder(this.id)
    if (!o) { this.setData({ o: null }); return }
    const st = S.stageOf(o)
    const mask = p => (o.privacy ? S.maskPhone(p) : p)
    const base = S.round2(o.fee - (o.insured ? 3 : 0))
    this.setData({
      st, tip: TIPS[st.key],
      steps: STEPS.map((t, i) => ({ t, on: st.idx >= i })),
      o: Object.assign({}, o, {
        carrierName: D.CARRIERS[o.carrier].name, initial: o.courier.name.charAt(0),
        senderPhone: mask(o.sender.phone), receiverPhone: mask(o.receiver.phone),
        baseText: base.toFixed(2), feeText: o.fee.toFixed(2), cutText: (o.cut || 0).toFixed(2), payText: o.pay.toFixed(2),
        time: S.stamp(o.createdAt)
      })
    })
  },
  onCopy() { host.copy(this.data.o.no, '运单号已复制') },
  onTrack() { wx.navigateTo({ url: '/pages/detail/detail?no=' + this.data.o.no }) },
  onCall() { host.call(this.data.o.courier.phone, '快递员 ' + this.data.o.courier.name) },
  onCancel() {
    const o = this.data.o
    wx.showModal({
      title: '取消寄件',
      content: '快递员还未上门，取消后运费 ¥' + o.payText + ' 原路退回' + (o.couponTitle ? '，「' + o.couponTitle + '」退回账户' : ''),
      confirmText: '取消寄件',
      cancelText: '再想想',
      confirmColor: '#f2453d',
      success: r => {
        if (!r.confirm) return
        if (!S.cancelOrder(o.id)) wx.showToast({ title: '快递员已揽收，不能取消', icon: 'none' })
        else wx.showToast({ title: '已取消，运费原路退回', icon: 'none' })
        this.load()
      }
    })
  },
  onAgain() {
    const o = this.data.o
    getApp().globalData.resend = { sender: o.sender, receiver: o.receiver, goods: o.goods, weight: o.weight, product: o.carrier, insured: o.insured }
    wx.switchTab({ url: '/pages/send/send' })
  },
  onOrders() { wx.redirectTo({ url: '/pages/orders/orders' }) }
})
