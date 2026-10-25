const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const SEAT_LETTERS = ['A', 'B', 'C', 'D', 'F']
const MAX_PASSENGERS = 5
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
    total: 0
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
    this.setData({ train: t, dateText: D.dateLabel(t.date), seatIdx, canChoose: t.type !== 'K' })
  },
  onShow() {
    const list = S.passengers()
    // 刚添加的乘车人自动勾上；首次进来默认勾本人
    if (this.known) list.forEach(p => { if (this.known.indexOf(p.id) < 0) this.checked[p.id] = true })
    else list.forEach(p => { if (p.self) this.checked[p.id] = true })
    this.known = list.map(p => p.id)
    this.applyPassengers(list)
  },
  applyPassengers(list) {
    const passengers = list.map(p => ({ id: p.id, name: p.name, idText: S.maskId(p.idcard), self: p.self, checked: !!this.checked[p.id] }))
    this.setData({ passengers })
    this.recalc()
  },
  // 总价 = 席别单价 × 乘车人数；人数变少时多选的座位从后往前丢
  recalc() {
    const count = this.data.passengers.filter(p => p.checked).length
    const price = this.data.train.seats[this.data.seatIdx].price
    this.setData({ count, price, total: price * count })
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
  onSubmit() {
    const { train, seatIdx, count, total } = this.data
    const seat = train.seats[seatIdx]
    if (!count) return wx.showToast({ title: '请选择乘车人', icon: 'none' })
    if (seat.left < count) return wx.showToast({ title: seat.name + '余票不足', icon: 'none' })
    const names = this.data.passengers.filter(p => p.checked).map(p => p.name)
    wx.showModal({
      title: '确认购票',
      content: train.no + ' ' + train.from + '→' + train.to + ' ' + this.data.dateText + ' ' + train.depText + '，' + seat.name + ' × ' + count + '（' + names.join('、') + '），合计 ¥' + total,
      confirmText: '支付',
      success: res => {
        if (!res.confirm) return
        S.addOrder({
          no: train.no, from: train.from, to: train.to, date: train.date, depText: train.depText, arrText: train.arrText,
          seat: seat.name, price: seat.price, count, total, names, seats: this.data.chosen.slice()
        })
        wx.showToast({ title: '购票成功', icon: 'success' })
        wx.switchTab({ url: '/pages/orders/orders' })
      }
    })
  }
})
