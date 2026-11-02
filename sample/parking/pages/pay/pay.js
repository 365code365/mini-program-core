const { COUPONS, lotById } = require('../../utils/data.js')
const F = require('../../utils/fee.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const Spot = require('../../utils/spot.js')

function nowHHMM() {
  const d = new Date()
  const pad = n => (n < 10 ? '0' : '') + n
  return pad(d.getHours()) + ':' + pad(d.getMinutes())
}

Page({
  data: {
    cars: [], plate: '', lot: null, inTime: '', now: '',
    minutes: 0, durText: '', fee: 0, feeText: '¥0.00',
    coupons: COUPONS.map(c => c.name), couponIdx: 0, off: 0, offText: '', pay: 0, payText: '¥0.00',
    tip: '', err: '', card: null
  },
  onShow() {
    // 用完的券在名字后标出来（下标不变：picker 的 value 仍对应 COUPONS）
    this.setData({ coupons: COUPONS.map((c, i) => (S.couponLeft(i) === 0 ? c.name + '（已用完）' : c.name)) })
  },
  onLoad(query) {
    // ?now=18:30 把「现在」钉死（测试 / 演示用），否则用系统时间
    this.setData({ now: /^\d{2}:\d{2}$/.test(query.now || '') ? query.now : nowHHMM() })
    this.loadCars(query.plate ? decodeURIComponent(query.plate) : '')
  },
  loadCars(want) {
    const cars = S.cars().map(c => ({ plate: c.plate, inside: !!S.sessionOf(c.plate) }))
    // 默认选带进来的车；没有就选第一辆在场的车
    const pick = cars.find(c => c.plate === want) || cars.find(c => c.inside) || cars[0]
    this.setData({ cars })
    this.pickCar(pick ? pick.plate : '')
  },
  pickCar(plate) {
    const s = plate ? S.sessionOf(plate) : null
    this.setData({ plate, lot: s ? lotById(s.lotId) : null, inTime: s ? s.inTime : '', err: '' })
    this.calc()
  },
  onCar(e) {
    this.pickCar(e.detail.value)
  },
  onIn(e) {
    // 道闸记录不准时手动校正入场时间
    this.setData({ inTime: e.detail.value })
    this.calc()
  },
  onCoupon(e) {
    this.setData({ couponIdx: Number(e.detail.value) })
    this.calc(true)
  },
  calc(fromPicker) {
    const d = this.data
    if (!d.lot) {
      return this.setData({ minutes: 0, durText: '', fee: 0, feeText: F.money(0), off: 0, offText: '', pay: 0, payText: F.money(0), tip: d.plate ? '该车辆当前不在场内' : '请先添加车辆' })
    }
    const minutes = F.between(d.inTime, d.now)
    const fee = F.calcFee(minutes, d.lot)
    // 月卡车：有效期内本场免费，不用券
    const card = S.cardFor(d.plate, d.lot.id)
    if (card) {
      if (fromPicker && d.couponIdx) wx.showToast({ title: '月卡车辆无需用券', icon: 'none' })
      return this.setData({
        minutes, durText: F.durText(minutes), fee, feeText: F.money(fee), couponIdx: 0, off: fee, offText: '', pay: 0, payText: F.money(0),
        card: { end: card.end }, tip: '月卡有效期至 ' + card.end + '，本次免费离场'
      })
    }
    let couponIdx = d.couponIdx
    let r = F.couponOff(COUPONS[couponIdx], fee, d.lot)
    if (r.ok && couponIdx && S.couponLeft(couponIdx) === 0) r = { off: 0, ok: false, why: '该券已用完' }
    if (!r.ok) {
      // 券不满足条件：退回「不使用」并提示
      if (fromPicker) wx.showToast({ title: r.why, icon: 'none' })
      couponIdx = 0
      r = { off: 0 }
    }
    const pay = Math.max(0, fee - r.off)
    this.setData({
      minutes, durText: F.durText(minutes), fee, feeText: F.money(fee),
      couponIdx, off: r.off, offText: r.off ? '-' + F.money(r.off) : '', pay, payText: F.money(pay), card: null,
      tip: fee === 0 ? '15 分钟内免费离场' : ''
    })
  },
  // 有钱要付：先调微信支付（宿主没有支付能力时弹确认框）；免费（15 分钟内 / 月卡）直接确认离场
  submit() {
    const d = this.data
    if (!d.lot) return this.fail(d.tip)
    const desc = d.plate + ' 在' + d.lot.name + '停车 ' + d.durText
    if (d.pay <= 0) {
      wx.showModal({
        title: '确认离场',
        content: desc + '，' + (d.card ? '月卡车辆' : '15 分钟内') + '免费，道闸将自动抬杆',
        confirmText: '离场',
        success: r => { if (r.confirm) this.leave() }
      })
      return
    }
    host.pay(d.pay, desc).then(ok => { if (ok) this.leave() })
  },
  leave() {
    const d = this.data
    const id = S.nextOrderId('P')
    const list = S.orders()
    const coupon = d.couponIdx ? COUPONS[d.couponIdx].name : ''
    list.unshift({
      id, type: 'pay', status: 'done', statusText: d.pay > 0 ? '已支付' : '免费离场', plate: d.plate, lotId: d.lot.id, lotName: d.lot.name,
      desc: d.inTime + ' 入场 · 停车 ' + d.durText, amount: d.pay, fee: d.fee, off: d.off, coupon: d.card ? '月卡' : coupon,
      inTime: d.inTime, outTime: d.now, payTime: S.stamp()
    })
    S.saveOrders(list)
    if (d.couponIdx) S.useCoupon(d.couponIdx)
    // 缴完费离场：在场记录清掉，车位还回去
    S.saveSessions(S.sessions().filter(s => s.plate !== d.plate))
    // 车开走了，寻车记录跟着清掉，下次入场重新记
    Spot.clear(d.plate)
    S.addFree(d.lot.id, 1)
    wx.redirectTo({ url: '/pages/result/result?id=' + id })
  },
  goCard() {
    wx.navigateTo({ url: '/pages/card/card?lot=' + this.data.lot.id + '&plate=' + encodeURIComponent(this.data.plate) })
  },
  fail(msg) {
    this.setData({ err: msg })
    wx.showToast({ title: msg, icon: 'none' })
  }
})
