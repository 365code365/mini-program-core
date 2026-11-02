const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
Page({
  data: {
    tab: 0,
    tabs: ['待出行', '已完成'],
    counts: [0, 0],
    list: []
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 1 })
    this.refresh()
    // 候补到点会兑现：页面开着时每 15 秒看一次
    clearInterval(this.timer)
    this.timer = setInterval(() => { if (S.sync()) this.refresh() }, 15000)
  },
  onHide() {
    clearInterval(this.timer)
  },
  onUnload() {
    clearInterval(this.timer)
  },
  refresh() {
    S.sync()
    const all = S.orders().map(o => {
      const st = S.orderState(o)
      const refund = o.status === 'cancelled' && o.refund ? '已退款 ¥' + o.refund.amount + (o.refund.fee ? '（手续费 ¥' + o.refund.fee + '）' : '') : ''
      return Object.assign({}, o, {
        done: st.done, stateText: st.text, dateText: D.dateLabel(o.date), namesText: (o.names || []).join('、'),
        seatsText: o.seats && o.seats.length ? o.seats.join(' ') : '', waiting: o.status === 'waiting',
        canChange: S.canChange(o), refundText: refund, changedText: o.changed ? '已改签（原 ' + o.changed.no + '）' : ''
      })
    })
    const todo = all.filter(o => !o.done)
    const done = all.filter(o => o.done)
    this.setData({ counts: [todo.length, done.length], list: this.data.tab === 0 ? todo : done })
  },
  onTab(e) {
    this.setData({ tab: Number(e.currentTarget.dataset.i) })
    this.refresh()
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id })
  },
  // 退票 / 取消候补：按开车前时长收手续费，退款原路返回
  onCancel(e) {
    const o = this.data.list[Number(e.currentTarget.dataset.index)]
    const q = S.refundQuote(o)
    const waiting = o.status === 'waiting'
    wx.showModal({
      title: waiting ? '取消候补' : '退票',
      content: waiting
        ? '取消 ' + o.no + '（' + o.dateText + '）的候补，预付款 ¥' + o.total + ' 全额原路退回'
        : '确定退掉 ' + o.no + '（' + o.dateText + '）的 ' + o.count + ' 张车票吗？' + q.rule + '，手续费 ¥' + q.fee + '，退款 ¥' + q.amount + ' 原路退回',
      confirmText: waiting ? '取消候补' : '确认退票',
      cancelText: '再想想',
      success: res => {
        if (!res.confirm) return
        const r = S.cancelOrder(o.id)
        this.refresh()
        wx.showToast({ title: r ? '已退款 ¥' + r.refund.amount : '订单状态已变化', icon: 'none' })
      }
    })
  },
  onChange(e) {
    wx.navigateTo({ url: '/pages/list/list?change=' + e.currentTarget.dataset.id })
  },
  // 再买一次：同一线路，日期从今天起
  onAgain(e) {
    const o = this.data.list[Number(e.currentTarget.dataset.index)]
    wx.navigateTo({ url: '/pages/list/list?from=' + o.from + '&to=' + o.to + '&date=' + (o.date < D.today() ? D.today() : o.date) })
  },
  onGo() {
    wx.switchTab({ url: '/pages/index/index' })
  }
})
