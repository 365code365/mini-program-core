const { lotById } = require('../../utils/data.js')
const S = require('../../utils/store.js')
const P = require('../../utils/spot.js')
const host = require('../../utils/host.js')
// 寻车：停好车记下楼层 / 区域 / 车位号（或扫柱子上的寻车码），回来时按记录给步行指引。
// 只列在场的车；带 plate=<车牌> 时默认选中那辆
Page({
  data: {
    cars: [], plates: [], idx: 0, car: null, lot: null, spot: null, guide: '', timeText: '',
    floors: P.FLOORS, zones: P.ZONES, floorIdx: 1, zoneIdx: 0, no: '', note: '', err: '', seq: 0, editing: false
  },
  onLoad(q) { this.want = q.plate ? decodeURIComponent(q.plate) : '' },
  onShow() {
    const cars = S.cars().map(c => ({ plate: c.plate, s: S.sessionOf(c.plate) })).filter(c => c.s)
    let idx = Math.max(0, cars.findIndex(c => c.plate === (this.want || (this.data.car && this.data.car.plate))))
    if (idx >= cars.length) idx = 0
    this.setData({ cars, plates: cars.map(c => c.plate + ' · ' + lotById(c.s.lotId).name), idx })
    this.pick(idx)
  },
  pick(idx) {
    const c = this.data.cars[idx]
    if (!c) { this.setData({ car: null, lot: null, spot: null }); return }
    const lot = lotById(c.s.lotId)
    const spot = P.of(c.plate)
    const road = !lot.month
    const patch = { idx, car: { plate: c.plate, inTime: c.s.inTime }, lot, road, err: '', editing: !spot, seq: this.data.seq + 1 }
    if (spot) Object.assign(patch, this.view(spot))
    else Object.assign(patch, { spot: null, floorIdx: road ? 3 : 1, zoneIdx: 0, no: '', note: '' })
    this.setData(patch)
  },
  view(spot) {
    const d = new Date(spot.at)
    const pad = n => (n < 10 ? '0' : '') + n
    return { spot, guide: P.guide(spot), timeText: pad(d.getHours()) + ':' + pad(d.getMinutes()) + ' 记录' }
  },
  onCar(e) { this.want = ''; this.pick(Number(e.detail.value)) },
  onFloor(e) { this.setData({ floorIdx: Number(e.detail.value) }) },
  onZone(e) { this.setData({ zoneIdx: Number(e.detail.value) }) },
  onNo(e) { this.setData({ no: e.detail.value.replace(/\D/g, '').slice(0, 4), err: '' }) },
  onNote(e) { this.setData({ note: e.detail.value }) },
  // 扫柱子上的寻车码；扫不了（桌面 / 无头）就提示手动填
  onScan() {
    wx.scanCode({
      onlyFromCamera: true,
      scanType: ['qrCode'],
      success: r => {
        const c = P.parseCode(r.result)
        if (!c) return wx.showToast({ title: '不是本停车场的寻车码', icon: 'none' })
        if (c.lotId !== this.data.lot.id) return wx.showToast({ title: '这个码属于其他停车场', icon: 'none' })
        this.setData({ floorIdx: P.FLOORS.indexOf(c.floor), zoneIdx: P.ZONES.indexOf(c.zone), no: c.no, err: '', seq: this.data.seq + 1 })
      },
      fail: err => {
        if (!/cancel/.test((err && err.errMsg) || '')) wx.showToast({ title: '当前环境不能扫码，请手动填写车位号', icon: 'none' })
      }
    })
  },
  onSave() {
    const d = this.data
    if (!d.no) return this.setData({ err: '请填写车位号（柱子或地面上的数字）' })
    if (Number(d.no) <= 0 || Number(d.no) > d.lot.total) return this.setData({ err: '车位号应在 1~' + d.lot.total + ' 之间（本场共 ' + d.lot.total + ' 个车位）' })
    const spot = P.save(d.car.plate, { floor: d.road ? '地面' : P.FLOORS[d.floorIdx], zone: P.ZONES[d.zoneIdx], no: d.no, note: d.note.trim() })
    this.setData(Object.assign({ editing: false, err: '' }, this.view(spot)))
    wx.showToast({ title: '已记下停车位置', icon: 'success' })
  },
  onEdit() {
    const s = this.data.spot
    this.setData({ editing: true, floorIdx: Math.max(0, P.FLOORS.indexOf(s.floor)), zoneIdx: Math.max(0, P.ZONES.indexOf(s.zone)), no: s.no, note: s.note || '', seq: this.data.seq + 1 })
  },
  onClear() {
    wx.showModal({
      title: '清除停车位置', content: '取到车了吗？清除后下次停车重新记录', confirmText: '清除',
      success: r => {
        if (!r.confirm) return
        P.clear(this.data.car.plate)
        this.pick(this.data.idx)
      }
    })
  },
  onNav() { host.openMap(this.data.lot) },
  onPay() { wx.navigateTo({ url: '/pages/pay/pay?plate=' + encodeURIComponent(this.data.car.plate) }) },
  goHome() { wx.navigateBack({ fail: () => wx.redirectTo({ url: '/pages/index/index' }) }) }
})
