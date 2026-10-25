const M = require('../../utils/market.js')
const A = require('../../utils/account.js')
const LOT = 100 // A 股一手 100 股
Page({
  data: {
    code: '',
    name: '',
    side: 'buy',
    q: null,
    price: 0,
    priceText: '',
    qty: LOT,
    maxQty: 0,
    sliderMax: LOT,
    amountText: '0.00',
    cash: 0,
    cashText: '',
    hold: 0,
    error: '',
    last: null
  },
  onLoad(query) {
    const code = M.info(query.code) ? query.code : '600519'
    const q = M.quote(code)
    this.setData({ code, name: q.name, side: query.side === 'sell' ? 'sell' : 'buy', price: q.price })
    this.recalc()
  },
  onTick() {
    // 行情只刷新参考价；委托价是用户定的，不跟着跳
    this.setData({ q: M.quote(this.data.code) })
  },
  recalc() {
    const cash = A.cash()
    const hold = A.holding(this.data.code)
    const p = this.data.price
    const maxQty = this.data.side === 'buy' ? Math.floor(cash / p / LOT) * LOT : hold
    let qty = this.data.qty
    if (qty < LOT) qty = LOT
    this.setData({
      q: M.quote(this.data.code),
      cash, hold, maxQty, qty,
      cashText: cash.toFixed(2),
      sliderMax: Math.max(LOT, maxQty),
      priceText: p.toFixed(2),
      amountText: M.r2(p * qty).toFixed(2)
    })
  },
  onSide(e) {
    this.setData({ side: e.currentTarget.dataset.s, qty: LOT, error: '' })
    this.recalc()
  },
  // 价格步进 ±0.01，不低于 0.01
  onPrice(e) {
    const d = Number(e.currentTarget.dataset.d)
    this.setData({ price: Math.max(0.01, M.r2(this.data.price + d)), error: '' })
    this.recalc()
  },
  onQtyStep(e) {
    const d = Number(e.currentTarget.dataset.d)
    this.setData({ qty: Math.max(LOT, this.data.qty + d), error: '' })
    this.recalc()
  },
  // slider：e.detail.value 是 number，step 已经是 100
  onSlide(e) {
    this.setData({ qty: Math.max(LOT, Math.round(e.detail.value / LOT) * LOT), error: '' })
    this.recalc()
  },
  onAll() {
    if (this.data.maxQty < LOT) return
    this.setData({ qty: this.data.maxQty })
    this.recalc()
  },
  validate() {
    const d = this.data
    if (d.qty % LOT !== 0) return '数量须为 100 的整数倍'
    if (d.side === 'buy' && M.r2(d.price * d.qty) > d.cash) return '可用资金不足'
    if (d.side === 'sell' && d.qty > d.hold) return d.hold ? '可卖数量不足' : '没有可卖的持仓'
    return ''
  },
  onSubmit() {
    const error = this.validate()
    if (error) {
      this.setData({ error })
      wx.showToast({ title: error, icon: 'none' })
      return
    }
    const d = this.data
    const verb = d.side === 'buy' ? '买入' : '卖出'
    wx.showModal({
      title: '确认' + verb,
      content: d.name + ' ' + d.qty + ' 股 @ ' + d.priceText + '，金额 ' + d.amountText + ' 元',
      confirmText: verb,
      confirmColor: d.side === 'buy' ? '#e93030' : '#12a35b',
      success: res => {
        if (!res.confirm) return
        const rec = A.execute(d.code, d.side, d.price, d.qty)
        this.setData({ last: rec, qty: LOT })
        this.recalc()
        wx.showToast({ title: '委托已成交', icon: 'success' })
      }
    })
  },
  goPositions() {
    wx.redirectTo({ url: '/pages/positions/positions' })
  }
})
