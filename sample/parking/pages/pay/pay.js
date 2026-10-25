const { COUPONS, lotById } = require('../../utils/data.js')
const F = require('../../utils/fee.js')
const S = require('../../utils/store.js')

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
    tip: '', err: ''
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
    let couponIdx = d.couponIdx
    let r = F.couponOff(COUPONS[couponIdx], fee, d.lot)
    if (!r.ok) {
      // 券不满足条件：退回「不使用」并提示
      if (fromPicker) wx.showToast({ title: r.why, icon: 'none' })
      couponIdx = 0
      r = { off: 0 }
    }
    const pay = Math.max(0, fee - r.off)
    this.setData({
      minutes, durText: F.durText(minutes), fee, feeText: F.money(fee),
      couponIdx, off: r.off, offText: r.off ? '-' + F.money(r.off) : '', pay, payText: F.money(pay),
      tip: fee === 0 ? '15 分钟内免费离场' : ''
    })
  },
  submit() {
    const d = this.data
    if (!d.lot) return this.fail(d.tip)
    wx.showModal({
      title: '确认支付',
      content: d.plate + ' 在' + d.lot.name + '停车 ' + d.durText + '，应付 ' + d.payText,
      confirmText: '支付',
      success: r => {
        if (!r.confirm) return
        const id = S.nextOrderId('P')
        const list = S.orders()
        list.unshift({
          id, type: 'pay', status: 'done', statusText: '已支付', plate: d.plate, lotId: d.lot.id, lotName: d.lot.name,
          desc: d.inTime + ' 入场 · 停车 ' + d.durText, amount: d.pay, fee: d.fee, off: d.off, coupon: d.couponIdx ? d.coupons[d.couponIdx] : ''
        })
        S.saveOrders(list)
        // 缴完费离场：在场记录清掉，车位还回去
        S.saveSessions(S.sessions().filter(s => s.plate !== d.plate))
        S.addFree(d.lot.id, 1)
        wx.redirectTo({ url: '/pages/result/result?id=' + id })
      }
    })
  },
  fail(msg) {
    this.setData({ err: msg })
    wx.showToast({ title: msg, icon: 'none' })
  }
})
