const { PROVINCES, ALNUM, lotById } = require('../../utils/data.js')
const S = require('../../utils/store.js')

// 车牌规则：首位省份简称、第二位字母；普通 7 位，新能源 8 位
const PLATE_RE = { 7: /^[\u4e00-\u9fa5][A-Z][A-Z0-9]{5}$/, 8: /^[\u4e00-\u9fa5][A-Z][A-Z0-9]{6}$/ }

Page({
  data: {
    plate: '',
    boxes: [],
    maxLen: 7,
    newEnergy: false,
    showKb: false,
    panel: 'prov',
    provKeys: PROVINCES,
    alnumKeys: ALNUM,
    err: '',
    cars: [],
    // 「点了下面的车辆卡片」的次数：键盘弹着时点键盘不该让它变（不穿透）
    carTaps: 0
  },
  onLoad() {
    this.syncPlate('')
  },
  onShow() {
    const list = S.cars().map(c => {
      const s = S.sessionOf(c.plate)
      const lot = s ? lotById(s.lotId) : null
      return { plate: c.plate, newEnergy: c.newEnergy, where: lot ? lot.name + ' · ' + s.inTime + ' 入场' : '未在场' }
    })
    this.setData({ cars: list })
  },
  syncPlate(plate) {
    const maxLen = this.data.newEnergy ? 8 : 7
    const boxes = []
    for (let i = 0; i < maxLen; i++) boxes.push({ ch: plate[i] || '', cur: this.data.showKb && i === plate.length })
    this.setData({ plate, boxes, maxLen, panel: plate.length === 0 ? 'prov' : 'alnum' })
  },
  openKb() {
    this.setData({ showKb: true, err: '' })
    this.syncPlate(this.data.plate)
  },
  closeKb() {
    this.setData({ showKb: false })
    this.syncPlate(this.data.plate)
  },
  noop() {},
  onKey(e) {
    const k = e.currentTarget.dataset.k
    const plate = this.data.plate
    if (plate.length >= this.data.maxLen) return
    // 第二位只能是字母
    if (plate.length === 1 && /[0-9]/.test(k)) return
    this.syncPlate(plate + k)
  },
  onDel() {
    this.syncPlate(this.data.plate.slice(0, -1))
  },
  toggleEnergy() {
    const ne = !this.data.newEnergy
    this.setData({ newEnergy: ne })
    // 从新能源切回普通：多出来的第 8 位截掉
    this.syncPlate(this.data.plate.slice(0, ne ? 8 : 7))
  },
  addCar() {
    const p = this.data.plate
    if (p.length < this.data.maxLen) return this.fail('车牌号不完整，还差 ' + (this.data.maxLen - p.length) + ' 位')
    if (!PLATE_RE[this.data.maxLen].test(p)) return this.fail('车牌格式不正确')
    const list = S.cars()
    if (list.some(c => c.plate === p)) return this.fail('已添加过 ' + p)
    if (list.length >= 5) return this.fail('最多绑定 5 辆车')
    list.push({ plate: p, newEnergy: this.data.newEnergy })
    S.saveCars(list)
    wx.showToast({ title: '已添加', icon: 'success' })
    this.setData({ showKb: false, err: '' })
    this.syncPlate('')
    this.onShow()
  },
  fail(msg) {
    this.setData({ err: msg })
    wx.showToast({ title: msg, icon: 'none' })
  },
  onCar(e) {
    this.setData({ carTaps: this.data.carTaps + 1 })
    wx.navigateTo({ url: '/pages/pay/pay?plate=' + encodeURIComponent(e.currentTarget.dataset.plate) })
  },
  delCar(e) {
    const plate = e.currentTarget.dataset.plate
    wx.showModal({
      title: '解绑车辆',
      content: '确定解绑 ' + plate + '？',
      confirmText: '解绑',
      confirmColor: '#f2453d',
      success: r => {
        if (!r.confirm) return
        S.saveCars(S.cars().filter(c => c.plate !== plate))
        this.onShow()
      }
    })
  },
  goLots() { wx.navigateTo({ url: '/pages/lots/lots' }) },
  goPay() { wx.navigateTo({ url: '/pages/pay/pay' }) },
  goOrders() { wx.navigateTo({ url: '/pages/orders/orders' }) }
})
