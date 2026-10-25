const D = require('../../utils/devices.js')

const MIN = 16
const MAX = 30
const WINDS = ['', '静音', '低风', '中风', '高风', '强劲']

Page({
  data: {
    dev: null,
    modes: D.MODES,
    timers: D.TIMERS,
    winds: WINDS,
    min: MIN,
    max: MAX
  },

  onLoad(query) {
    this.id = query.id || 'ac1'
    const dev = D.get(this.id)
    this.setData({ dev })
    if (dev) wx.setNavigationBarTitle({ title: dev.name })
  },

  // 改一项：画面与 storage 一起改
  set(patch) {
    const dev = D.update(this.id, patch)
    this.setData({ dev })
  },

  onPower() {
    this.set({ on: !this.data.dev.on })
  },

  // 温度 ±：16~30 封顶，送风模式不能调温
  onStep(e) {
    const d = this.data.dev
    if (!d.on) {
      wx.showToast({ title: '请先打开空调', icon: 'none' })
      return
    }
    if (d.mode === 'fan') {
      wx.showToast({ title: '送风模式不能调温', icon: 'none' })
      return
    }
    const t = d.temp + Number(e.currentTarget.dataset.d)
    if (t < MIN || t > MAX) {
      wx.showToast({ title: t < MIN ? '已是最低 ' + MIN + '℃' : '已是最高 ' + MAX + '℃', icon: 'none' })
      return
    }
    this.set({ temp: t })
  },

  onMode(e) {
    this.set({ mode: e.detail.value })
  },

  // 拖动中只改画面，松手才落盘
  onWindChanging(e) {
    this.setData({ 'dev.wind': e.detail.value })
  },

  onWind(e) {
    this.set({ wind: e.detail.value })
  },

  onTimer(e) {
    const i = Number(e.detail.value)
    this.set({ timer: i })
    if (i) wx.showToast({ title: '将在' + D.TIMERS[i].replace('后关', '') + '后关闭', icon: 'none' })
  }
})
