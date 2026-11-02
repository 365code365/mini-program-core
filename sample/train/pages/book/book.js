const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const SEAT_LETTERS = ['A', 'B', 'C', 'D', 'F']
const MAX_PASSENGERS = 5
// 三种用法：普通购票；选的席别没票 → 候补（预付票款，有票自动出票）；带 change=<订单号> → 改签（乘车人固定，只换车次 / 席别）
Page({
  data: {
    train: null,
    dateText: '',
    seatIdx: 0,
    passengers: [],
    count: 0,
    letters: SEAT_LETTERS,
    chosen: [],
    seatOn: {},
    canChoose: true,
    price: 0,
    total: 0,
    wait: false,
    discountText: '',
    change: null,
    diff: 0,
    diffText: ''
  },
  onLoad(q) {
    let t = getApp().globalData.train
    // 直接打开（分享 / 冒烟）：按默认查询条件找这趟车
    if (!t || (q.no && t.no !== q.no)) {
      const list = D.trains('北京', '上海', D.today())
      t = list.find(x => x.no === q.no) || list[0]
    }
    const seatIdx = Math.max(0, t.seats.findIndex(s => s.left > 0))
    this.checked = {}
    const patch = { train: t, dateText: D.dateLabel(t.date), seatIdx, canChoose: t.type !== 'K' }
    const o = q.change ? S.findOrder(q.change) : null
    if (o && S.canChange(o)) {
      this.order = o
      patch.change = { no: o.no, dateText: D.dateLabel(o.date), depText: o.depText, seat: o.seat, total: o.total }
      wx.setNavigationBarTitle({ title: '确认改签' })
    }
    this.setData(patch)
  },
  onShow() {
    if (this.order) {
      // 改签：乘车人就是原订单上的那几位，不能增减
      const tickets = S.ticketList(this.order)
      this.setData({ passengers: tickets.map((t, i) => ({ id: 'c' + i, name: t.name, type: t.type, idText: t.text, self: false, checked: true })) })
      this.recalc()
      return
    }
    const list = S.passengers()
    // 刚添加的乘车人自动勾上；首次进来默认勾本人；删掉的乘车人去掉勾
    if (this.known) list.forEach(p => { if (this.known.indexOf(p.id) < 0) this.checked[p.id] = true })
    else list.forEach(p => { if (p.self) this.checked[p.id] = true })
    this.known = list.map(p => p.id)
    this.applyPassengers(list)
  },
  applyPassengers(list) {
    const passengers = list.map(p => ({ id: p.id, name: p.name, idText: S.maskId(p.idcard), self: p.self, type: p.type || '成人票', checked: !!this.checked[p.id] }))
    this.setData({ passengers })
    this.recalc()
  },
  // 总价 = 每位乘车人按票种算的票价之和（成人全价）；人数变少时多选的座位从后往前丢
  recalc() {
    const seat = this.data.train.seats[this.data.seatIdx]
    const picked = this.data.passengers.filter(p => p.checked)
    const count = picked.length
    const total = picked.reduce((s, p) => s + S.ticketPrice(seat, p.type), 0)
    const kinds = {}
    picked.forEach(p => { if (p.type && p.type !== '成人票' && S.typeRate(p.type, seat.name) < 1) kinds[p.type] = (kinds[p.type] || 0) + 1 })
    const cut = seat.price * count - total
    const patch = {
      count, price: seat.price, total, wait: !this.order && seat.left === 0,
      discountText: cut > 0 ? '含' + Object.keys(kinds).map(k => k + ' ' + kinds[k] + ' 张').join('、') + '，优惠 ¥' + cut : ''
    }
    if (this.order) {
      const diff = total - this.order.total
      patch.diff = diff
      patch.diffText = diff > 0 ? '需补差价 ¥' + diff : diff < 0 ? '退还差价 ¥' + -diff : '票价不变'
    }
    this.setData(patch)
    this.setChosen(this.data.chosen.slice(0, count))
  },
  // radio-group：e.detail.value 是 radio 的 value（字符串）
  onSeat(e) {
    this.setData({ seatIdx: Number(e.detail.value) })
    this.recalc()
  },
  // checkbox-group：e.detail.value 是选中的 value 数组（字符串）
  onPassengers(e) {
    const ids = e.detail.value.map(Number)
    if (ids.length > MAX_PASSENGERS) {
      wx.showToast({ title: '每单最多 ' + MAX_PASSENGERS + ' 位乘车人', icon: 'none' })
      this.applyPassengers(S.passengers())
      return
    }
    this.checked = {}
    ids.forEach(id => { this.checked[id] = true })
    this.applyPassengers(S.passengers())
  },
  onStops() {
    const t = this.data.train
    wx.navigateTo({ url: '/pages/stops/stops?no=' + t.no + '&from=' + encodeURIComponent(t.from) + '&to=' + encodeURIComponent(t.to) + '&date=' + t.date + (this.order ? '' : '&book=1') })
  },
  onAddPassenger() {
    wx.navigateTo({ url: '/pages/passenger/passenger' })
  },
  // 选座：每位乘车人一个座位，点已选的取消；选满了再点就顶掉最早选的
  onLetter(e) {
    const l = e.currentTarget.dataset.l
    if (!this.data.count) {
      wx.showToast({ title: '请先选择乘车人', icon: 'none' })
      return
    }
    let chosen = this.data.chosen.slice()
    const i = chosen.indexOf(l)
    if (i >= 0) chosen.splice(i, 1)
    else {
      chosen.push(l)
      if (chosen.length > this.data.count) chosen = chosen.slice(chosen.length - this.data.count)
    }
    this.setChosen(chosen)
  },
  setChosen(chosen) {
    const seatOn = {}
    chosen.forEach(l => { seatOn[l] = true })
    this.setData({ chosen, seatOn })
  },
  onWaitRule() {
    wx.showModal({
      title: '候补购票说明',
      content: '所选席别暂时无票时可提交候补并预付票款。有旅客退票或加开车厢时，系统按提交顺序自动出票；开车前仍未兑现的，预付款全额原路退回。候补期间可随时在订单里取消，不收手续费。',
      showCancel: false,
      confirmText: '知道了'
    })
  },
  // 提交 = 支付：微信里弹微信支付；宿主没有支付能力时退回页内确认框
  onSubmit() {
    if (this.paying) return
    if (this.order) return this.submitChange()
    const { train, seatIdx, count, total } = this.data
    const seat = train.seats[seatIdx]
    if (!count) return wx.showToast({ title: '请选择乘车人', icon: 'none' })
    const wait = seat.left === 0
    if (!wait && seat.left < count) return wx.showToast({ title: seat.name + '余票不足，仅剩 ' + seat.left + ' 张', icon: 'none' })
    const picked = this.data.passengers.filter(p => p.checked)
    const names = picked.map(p => p.name)
    const desc = (wait ? '候补预付 · ' : '') + train.no + ' ' + train.from + '→' + train.to + ' ' + this.data.dateText + ' ' + train.depText + '\n' + seat.name + ' × ' + count + '（' + names.join('、') + '）'
    this.paying = true
    host.pay(total, desc).then(ok => {
      this.paying = false
      if (!ok) return wx.showToast({ title: '已取消支付', icon: 'none' })
      S.addOrder({
        no: train.no, from: train.from, to: train.to, date: train.date, depText: train.depText, arrText: train.arrText,
        seat: seat.name, price: seat.price, prices: picked.map(p => S.ticketPrice(seat, p.type)), count, total,
        names, types: picked.map(p => p.type), seats: wait ? [] : this.data.chosen.slice(), status: wait ? 'waiting' : 'paid'
      })
      wx.showToast({ title: wait ? '候补已提交' : '购票成功', icon: 'success' })
      wx.switchTab({ url: '/pages/orders/orders' })
    })
  },
  submitChange() {
    const { train, seatIdx, count, total, diff } = this.data
    const seat = train.seats[seatIdx]
    const o = this.order
    if (train.no === o.no && train.date === o.date && seat.name === o.seat) return wx.showToast({ title: '与原车票相同，请换个车次或席别', icon: 'none' })
    if (seat.left < count) return wx.showToast({ title: seat.name + '余票不足', icon: 'none' })
    const n = {
      no: train.no, date: train.date, depText: train.depText, arrText: train.arrText, seat: seat.name, price: seat.price,
      prices: this.data.passengers.map(p => S.ticketPrice(seat, p.type)), total, seats: this.data.chosen.slice()
    }
    const done = () => {
      S.changeOrder(o.id, n)
      wx.showToast({ title: '改签成功', icon: 'success' })
      // 回到订单详情（详情 → 车次列表 → 这里）；从订单列表直接来的回订单 tab
      const pages = getCurrentPages()
      const i = pages.map(p => p.route).lastIndexOf('pages/order-detail/order-detail')
      if (i >= 0) wx.navigateBack({ delta: pages.length - 1 - i })
      else wx.switchTab({ url: '/pages/orders/orders' })
    }
    if (diff > 0) {
      this.paying = true
      host.pay(diff, '改签补差价 · ' + o.no + ' → ' + train.no + ' ' + seat.name).then(ok => {
        this.paying = false
        if (ok) done()
        else wx.showToast({ title: '已取消支付', icon: 'none' })
      })
      return
    }
    wx.showModal({
      title: '确认改签',
      content: o.no + ' ' + D.dateLabel(o.date) + ' 改为 ' + train.no + ' ' + this.data.dateText + ' ' + train.depText + ' ' + seat.name + '，' + (diff < 0 ? '差价 ¥' + -diff + ' 原路退回' : '票价不变') + '。改签后不能再次改签。',
      confirmText: '确认改签',
      success: r => { if (r.confirm) done() }
    })
  }
})
