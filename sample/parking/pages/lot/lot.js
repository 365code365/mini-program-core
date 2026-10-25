const { DURATIONS, lotById } = require('../../utils/data.js')
const F = require('../../utils/fee.js')
const S = require('../../utils/store.js')

Page({
  data: {
    lot: null, free: 0, rules: [], plates: [], carIdx: 0,
    arrive: '18:00', durations: DURATIONS.map(d => d.text), durIdx: 1,
    fee: 0, feeText: '', leaveText: '', err: ''
  },
  onLoad(query) {
    const lot = lotById(query.id || 1)
    this.setData({ lot, rules: F.ruleText(lot), plates: S.cars().map(c => c.plate) })
    wx.setNavigationBarTitle({ title: lot.name })
    this.calc()
  },
  onShow() {
    if (this.data.lot) this.setData({ free: S.freeOf(this.data.lot.id) })
  },
  // 预约费用 = 按预约时长套同一套计费规则
  calc() {
    const hours = DURATIONS[this.data.durIdx].hours
    const fee = F.calcFee(hours * 60, this.data.lot)
    const leave = (F.toMin(this.data.arrive) + hours * 60) % 1440
    const pad = n => (n < 10 ? '0' : '') + n
    this.setData({ fee, feeText: F.money(fee), leaveText: pad(Math.floor(leave / 60)) + ':' + pad(leave % 60) })
  },
  onArrive(e) {
    this.setData({ arrive: e.detail.value })
    this.calc()
  },
  onDur(e) {
    this.setData({ durIdx: Number(e.detail.value) })
    this.calc()
  },
  onCar(e) {
    this.setData({ carIdx: Number(e.detail.value) })
  },
  reserve() {
    const d = this.data
    if (d.free <= 0) return this.fail('暂无空位，无法预约')
    if (!d.plates.length) return this.fail('请先在首页添加车辆')
    const plate = d.plates[d.carIdx]
    wx.showModal({
      title: '确认预约',
      content: plate + ' 于 ' + d.arrive + ' 到达，保留 ' + d.durations[d.durIdx] + '，预计 ' + d.feeText,
      confirmText: '预约',
      success: r => {
        if (!r.confirm) return
        const list = S.orders()
        list.unshift({
          id: S.nextOrderId('R'), type: 'reserve', status: 'ongoing', statusText: '已预约', plate,
          lotId: d.lot.id, lotName: d.lot.name, desc: d.arrive + ' 到达 · 保留' + d.durations[d.durIdx], amount: d.fee
        })
        S.saveOrders(list)
        S.addFree(d.lot.id, -1)
        this.setData({ free: S.freeOf(d.lot.id), err: '' })
        wx.navigateTo({ url: '/pages/orders/orders?tab=0' })
      }
    })
  },
  fail(msg) {
    this.setData({ err: msg })
    wx.showToast({ title: msg, icon: 'none' })
  }
})
