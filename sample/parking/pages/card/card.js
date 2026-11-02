// 月卡：我的月卡（有效期、续费）+ 办理（停车场、车辆、月 / 季 / 年卡）。有效期内在该停车场缴费页直接免费离场
const { LOTS, PLANS, lotById } = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const CARD_LOTS = LOTS.filter(l => l.month > 0)
Page({
  data: {
    mine: [], lotNames: CARD_LOTS.map(l => l.name), lotIdx: 0, plates: [], carIdx: 0,
    plans: [], planIdx: 0, priceText: '', endText: ''
  },
  onLoad(q) {
    const li = CARD_LOTS.findIndex(l => l.id === Number(q.lot))
    const plates = S.cars().map(c => c.plate)
    const ci = plates.indexOf(q.plate ? decodeURIComponent(q.plate) : '')
    this.setData({ lotIdx: li >= 0 ? li : 0, carIdx: ci >= 0 ? ci : 0 })
  },
  onShow() {
    const t = S.today()
    this.setData({
      plates: S.cars().map(c => c.plate),
      mine: S.cards().map(c => Object.assign({}, c, { valid: c.end >= t }))
    })
    this.calc()
  },
  calc() {
    const lot = CARD_LOTS[this.data.lotIdx]
    const plans = PLANS.map(p => ({ key: p.key, name: p.name, days: p.days, price: S.planPrice(lot, p) }))
    const p = plans[this.data.planIdx]
    this.setData({ plans, priceText: '¥' + p.price.toFixed(2) })
  },
  onLot(e) { this.setData({ lotIdx: Number(e.detail.value) }); this.calc() },
  onCar(e) { this.setData({ carIdx: Number(e.detail.value) }) },
  onPlan(e) { this.setData({ planIdx: Number(e.currentTarget.dataset.i) }); this.calc() },
  buy(plate, lotId, planKey) {
    const lot = lotById(lotId)
    const plan = PLANS.find(p => p.key === planKey)
    const price = S.planPrice(lot, plan)
    host.pay(price, plate + ' 办理' + lot.name + plan.name).then(ok => {
      if (!ok) return
      const r = S.buyCard(plate, lotId, planKey)
      this.onShow()
      wx.showToast({ title: plan.name + '已生效，有效期至 ' + r.card.end, icon: 'none' })
    })
  },
  onBuy() {
    const d = this.data
    if (!d.plates.length) return wx.showToast({ title: '请先在首页添加车辆', icon: 'none' })
    this.buy(d.plates[d.carIdx], CARD_LOTS[d.lotIdx].id, d.plans[d.planIdx].key)
  },
  // 续费：沿用这张卡的车和停车场，选套餐
  onRenew(e) {
    const c = this.data.mine[Number(e.currentTarget.dataset.i)]
    const lot = lotById(c.lotId)
    wx.showActionSheet({
      itemList: PLANS.map(p => p.name + ' ¥' + S.planPrice(lot, p)),
      success: r => this.buy(c.plate, c.lotId, PLANS[r.tapIndex].key)
    })
  },
  goLot(e) {
    const c = this.data.mine[Number(e.currentTarget.dataset.i)]
    wx.navigateTo({ url: '/pages/lot/lot?id=' + c.lotId })
  }
})
