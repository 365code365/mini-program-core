// 寄快递：地址簿选寄 / 收件人 → 物品与重量 → 快递产品 → 上门时间 → 保价 / 留言 → 用券 → 预付运费下单
const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const DAYS = ['今天', '明天', '后天']
const SLOTS = ['一小时内', '14:00-16:00', '16:00-18:00', '18:00-20:00']
const INSURE_FEE = 3
function view(a) {
  return a ? Object.assign({}, a, { phoneText: S.maskPhone(a.phone) }) : null
}
Page({
  data: {
    sender: null,
    receiver: null,
    goodsList: ['日用品', '数码产品', '衣物', '食品', '文件', '其他'],
    goods: 0,
    weight: 2,
    carrierList: D.PRODUCTS,
    carrier: 0,
    timeRange: [DAYS, SLOTS],
    timeIndex: [0, 0],
    timeText: '今天 一小时内',
    insured: false,
    note: '',
    noteSeq: 0,
    fee: 0,
    first: 0,
    extra: 0,
    coupons: [],
    couponNames: [],
    couponIdx: 0,
    cut: 0,
    pay: 0
  },
  onLoad() {
    const def = S.defaultAddr()
    const l = S.getAddrs()
    this.senderId = def ? def.id : ''
    this.receiverId = (l.find(a => a.id !== this.senderId && a.name !== (def && def.name)) || {}).id || ''
    // 用券：'' = 自动选最省的，'none' = 不用，其余是券 id
    this.couponPick = ''
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 })
    const app = getApp()
    const picked = app.globalData.pickedAddress
    if (picked) {
      this[picked.role + 'Id'] = picked.addr.id
      app.globalData.pickedAddress = null
    }
    // 「再寄一单」：带着上一单的人、物、产品过来
    const re = app.globalData.resend
    if (re) {
      app.globalData.resend = null
      this.senderId = re.sender.id
      this.receiverId = re.receiver.id
      const ci = D.PRODUCTS.findIndex(p => p.key === re.product)
      this.setData({
        goods: Math.max(0, this.data.goodsList.indexOf(re.goods)), weight: re.weight,
        carrier: ci >= 0 ? ci : 0, insured: !!re.insured
      })
    }
    // 地址簿里改过 / 删掉的地址：按 id 重新取
    this.setData({ sender: view(S.findAddr(this.senderId)), receiver: view(S.findAddr(this.receiverId)) })
    this.calc()
  },
  calc() {
    const c = D.PRODUCTS[this.data.carrier]
    const w = Number(this.data.weight)
    const fee = c.first + (w - 1) * c.extra + (this.data.insured ? INSURE_FEE : 0)
    const usable = S.usableCoupons(fee)
    let pick = null
    if (this.couponPick === '') pick = S.bestCoupon(fee)
    else if (this.couponPick !== 'none') pick = usable.find(x => x.id === this.couponPick) || S.bestCoupon(fee)
    const cut = pick ? Math.min(pick.amount, fee) : 0
    this.setData({
      fee, first: c.first, extra: c.extra,
      coupons: usable,
      couponNames: ['不使用优惠券'].concat(usable.map(x => x.title + ' -¥' + x.amount)),
      couponIdx: pick ? usable.indexOf(pick) + 1 : 0,
      cut, pay: S.round2(fee - cut)
    })
  },
  onPick(e) {
    wx.navigateTo({ url: '/pages/address/address?select=' + e.currentTarget.dataset.role })
  },
  onGoods(e) {
    this.setData({ goods: Number(e.currentTarget.dataset.i) })
  },
  onWeight(e) {
    this.setData({ weight: Number(e.detail.value) })
    this.calc()
  },
  onStep(e) {
    const w = Math.min(30, Math.max(1, Number(this.data.weight) + Number(e.currentTarget.dataset.d)))
    this.setData({ weight: w })
    this.calc()
  },
  onCarrier(e) {
    this.setData({ carrier: Number(e.currentTarget.dataset.i) })
    this.calc()
  },
  onTime(e) {
    const v = e.detail.value.map(Number)
    // 「一小时内」只有今天能选
    if (v[0] > 0 && v[1] === 0) v[1] = 1
    this.setData({ timeIndex: v, timeText: DAYS[v[0]] + ' ' + SLOTS[v[1]] })
  },
  onInsure(e) {
    this.setData({ insured: !!e.detail.value })
    this.calc()
  },
  onNote(e) {
    this.setData({ note: e.detail.value })
  },
  onCoupon(e) {
    const i = Number(e.detail.value)
    this.couponPick = i === 0 ? 'none' : this.data.coupons[i - 1].id
    this.calc()
  },
  onSubmit() {
    const { sender, receiver } = this.data
    if (!sender || !receiver) {
      wx.showToast({ title: '请先填写寄收件人', icon: 'none' })
      return
    }
    if (sender.id === receiver.id) {
      wx.showToast({ title: '寄件人和收件人不能是同一个地址', icon: 'none' })
      return
    }
    const d = this.data
    const c = D.PRODUCTS[d.carrier]
    const coupon = d.couponIdx ? d.coupons[d.couponIdx - 1] : null
    host.pay(d.pay, c.name + ' · ' + d.timeText + ' 上门\n' + S.cityOf(sender) + ' → ' + S.cityOf(receiver) + '，' + d.weight + 'kg ' + d.goodsList[d.goods]).then(ok => {
      if (!ok) return
      const strip = a => ({ id: a.id, name: a.name, phone: a.phone, region: a.region, detail: a.detail })
      const o = S.createOrder({
        product: c.key, sender: strip(sender), receiver: strip(receiver), goods: d.goodsList[d.goods], weight: Number(d.weight),
        timeText: d.timeText, insured: d.insured, note: d.note.trim(), fee: d.fee, couponId: coupon ? coupon.id : '',
        couponTitle: coupon ? coupon.title : '', cut: d.cut, pay: d.pay, privacy: S.settings().privacy
      })
      this.couponPick = ''
      this.setData({ note: '', noteSeq: d.noteSeq + 1 })
      this.calc()
      wx.showToast({ title: '下单成功', icon: 'success' })
      wx.navigateTo({ url: '/pages/order/order?id=' + o.id })
    })
  }
})
