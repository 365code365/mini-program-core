const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
Page({
  data: {
    order: null,
    tip: '',
    st: null,
    tickets: [],
    gate: '',
    quote: null,
    created: '',
    waiting: false,
    canChange: false,
    waitLeft: 0,
    lines: []
  },
  onLoad(q) {
    this.id = q.id || ''
  },
  onShow() {
    this.load()
    clearInterval(this.timer)
    this.timer = setInterval(() => this.load(), 10000)
  },
  onHide() {
    clearInterval(this.timer)
  },
  onUnload() {
    clearInterval(this.timer)
  },
  load() {
    S.sync()
    const o = S.findOrder(this.id)
    if (!o) {
      this.setData({ order: null })
      return
    }
    const st = S.orderState(o)
    const lines = []
    if (o.changed) {
      lines.push({ name: '原车票（' + o.changed.no + ' ' + o.changed.seat + '）', value: '¥' + o.changed.total })
      lines.push({ name: o.changed.diff >= 0 ? '改签补差价' : '改签退差价（已原路退回）', value: (o.changed.diff >= 0 ? '+¥' : '-¥') + Math.abs(o.changed.diff), cut: o.changed.diff < 0 })
    }
    lines.push({ name: (o.changed ? '现车票' : '票价') + '（' + o.seat + ' × ' + o.count + '）', value: '¥' + o.total })
    if (o.refund) {
      lines.push({ name: '退票手续费', value: '¥' + o.refund.fee })
      lines.push({ name: '退款金额（原路退回）', value: '¥' + o.refund.amount, cut: true })
    }
    let tip = '请携带购票时使用的有效身份证件，开车前到' + S.gateOf(o) + '检票'
    if (o.status === 'waiting') tip = '候补排队中，有旅客退票时按提交顺序自动出票'
    else if (o.status === 'cancelled') tip = o.refund && o.refund.waiting ? '候补已取消，预付款已全额原路退回' : '已退票，退款 1~7 个工作日原路到账'
    else if (st.done) tip = '旅途愉快，欢迎再次乘车'
    this.setData({
      tip,
      order: Object.assign({}, o, { dateText: D.dateLabel(o.date), namesText: (o.names || []).join('、'), seatsText: (o.seats || []).join(' ') }),
      st,
      waiting: o.status === 'waiting',
      tickets: S.ticketList(o),
      gate: S.gateOf(o),
      quote: st.done ? null : S.refundQuote(o),
      canChange: S.canChange(o),
      created: o.created ? S.dateTime(o.created) : o.date,
      waitLeft: o.status === 'waiting' ? Math.max(1, Math.ceil((S.FULFILL_MS - (Date.now() - (o.created || 0))) / 1000)) : 0,
      lines
    })
  },
  onCopy() {
    host.copy(this.data.order.id)
  },
  onService() {
    host.call('12306')
  },
  onStops() {
    const o = this.data.order
    wx.navigateTo({ url: '/pages/stops/stops?no=' + o.no + '&from=' + encodeURIComponent(o.from) + '&to=' + encodeURIComponent(o.to) + '&date=' + o.date })
  },
  onRule() {
    wx.showModal({
      title: '退改签规则',
      content: '退票：开车前 8 天以上免费；48 小时~8 天收 5%；24~48 小时收 10%；不足 24 小时收 20%，每张最低 2 元。\n改签：开车前可在线改签一次，新票价高补差价、低退差价。\n候补：未兑现前随时取消，全额退款。',
      showCancel: false,
      confirmText: '知道了'
    })
  },
  onRefund() {
    const o = this.data.order
    const q = S.refundQuote(o)
    const waiting = o.status === 'waiting'
    wx.showModal({
      title: waiting ? '取消候补' : '退票',
      content: waiting ? '预付款 ¥' + o.total + ' 将全额原路退回' : q.rule + '，手续费 ¥' + q.fee + '，退款 ¥' + q.amount + ' 将在 1~7 个工作日原路退回',
      confirmText: waiting ? '取消候补' : '确认退票',
      cancelText: '再想想',
      success: r => {
        if (!r.confirm) return
        const done = S.cancelOrder(o.id)
        wx.showToast({ title: done ? '已退款 ¥' + done.refund.amount : '订单状态已变化', icon: 'none' })
        this.load()
      }
    })
  },
  onChange() {
    wx.navigateTo({ url: '/pages/list/list?change=' + this.data.order.id })
  },
  // 再买一次 / 买返程：带上线路去车次列表
  onAgain(e) {
    const o = this.data.order
    // data-back="1"：微信按字面量还原成数字 1（文档示例 data-alpha-beta="1" → === 1），两种都认
    const back = String(e.currentTarget.dataset.back) === '1'
    const from = back ? o.to : o.from
    const to = back ? o.from : o.to
    const date = back ? D.addDays(o.date < D.today() ? D.today() : o.date, 1) : (o.date < D.today() ? D.today() : o.date)
    wx.navigateTo({ url: '/pages/list/list?from=' + from + '&to=' + to + '&date=' + date })
  },
  onGoOrders() {
    wx.switchTab({ url: '/pages/orders/orders' })
  }
})
