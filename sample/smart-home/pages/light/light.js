const D = require('../../utils/devices.js')

// 预设颜色块
const COLORS = [
  { v: '#ffffff', name: '纯白' },
  { v: '#ffe8b8', name: '暖黄' },
  { v: '#ffb4a2', name: '落日' },
  { v: '#b8e0ff', name: '冰蓝' },
  { v: '#c8b6ff', name: '薰衣草' },
  { v: '#b9fbc0', name: '薄荷' }
]

// 色温 → 预览光晕的颜色（2700K 偏橙、6500K 偏蓝白），线性插值
function ctColor(k) {
  const t = (k - 2700) / (6500 - 2700)
  const mix = (a, b) => Math.round(a + (b - a) * t)
  return 'rgb(' + mix(255, 220) + ',' + mix(196, 234) + ',' + mix(120, 255) + ')'
}

Page({
  data: {
    dev: null,
    colors: COLORS,
    glow: '',
    colorName: ''
  },

  onLoad(query) {
    this.id = query.id || 'light1'
    this.apply(D.get(this.id))
    if (this.data.dev) wx.setNavigationBarTitle({ title: this.data.dev.name })
  },

  apply(dev) {
    const c = COLORS.find(x => x.v === dev.color)
    this.setData({ dev, glow: ctColor(dev.ct), colorName: c ? c.name : '自定义' })
  },

  set(patch) {
    this.apply(D.update(this.id, patch))
  },

  onPower() {
    this.set({ on: !this.data.dev.on })
  },

  onBrightChanging(e) {
    this.setData({ 'dev.brightness': e.detail.value })
  },

  // 调亮度时灯是关的就顺手打开（与主流 App 一致）
  onBright(e) {
    this.set({ brightness: e.detail.value, on: true })
  },

  onCtChanging(e) {
    this.setData({ 'dev.ct': e.detail.value, glow: ctColor(e.detail.value) })
  },

  onCt(e) {
    this.set({ ct: e.detail.value, on: true })
  },

  onColor(e) {
    this.set({ color: e.currentTarget.dataset.v, on: true })
  }
})
