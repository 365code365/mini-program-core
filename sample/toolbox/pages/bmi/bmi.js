// BMI 分级按《中国成人超重和肥胖预防控制指南》：<18.5 偏瘦，18.5~23.9 正常，24~27.9 超重，≥28 肥胖
const LEVELS = [
  { max: 18.5, name: '偏瘦', range: '< 18.5', color: '#4c9bff', tip: '体重偏轻，注意均衡饮食，适当增加优质蛋白与力量训练。' },
  { max: 24, name: '正常', range: '18.5 ~ 23.9', color: '#2fbf8f', tip: '体重在健康范围，保持规律作息与每周 150 分钟中等强度运动。' },
  { max: 28, name: '超重', range: '24 ~ 27.9', color: '#f5a623', tip: '略高于健康范围，建议控制精制碳水与油脂，每天快走 30 分钟。' },
  { max: 999, name: '肥胖', range: '≥ 28', color: '#f04848', tip: '已达到肥胖标准，建议咨询医生或营养师，制定科学减重计划。' }
]
const SCALE_MIN = 14
const SCALE_MAX = 34

Page({
  data: { height: 172, weight: 65, gender: 0, bmi: '0', level: LEVELS[1], pos: 0, idealLow: '0', idealHigh: '0', levels: LEVELS },
  onLoad() {
    const s = wx.getStorageSync('tb_bmi')
    if (s) this.setData({ height: s.h, weight: s.w, gender: s.g || 0 })
    this.calc()
  },
  calc() {
    const h = this.data.height / 100
    const b = this.data.weight / (h * h)
    let level = LEVELS[LEVELS.length - 1]
    for (let i = 0; i < LEVELS.length; i++) if (b < LEVELS[i].max) { level = LEVELS[i]; break }
    const pos = Math.max(0, Math.min(100, (b - SCALE_MIN) / (SCALE_MAX - SCALE_MIN) * 100))
    this.setData({
      bmi: b.toFixed(1), level, pos: Math.round(pos * 10) / 10,
      idealLow: (18.5 * h * h).toFixed(1), idealHigh: (23.9 * h * h).toFixed(1)
    })
    wx.setStorageSync('tb_bmi', { h: this.data.height, w: this.data.weight, g: this.data.gender })
  },
  onH(e) { this.setData({ height: e.detail.value }); this.calc() },
  onW(e) { this.setData({ weight: e.detail.value }); this.calc() },
  onGender(e) { this.setData({ gender: +e.currentTarget.dataset.g }); this.calc() },
  onStep(e) {
    const { k, d } = e.currentTarget.dataset
    const lim = k === 'height' ? [120, 220] : [30, 160]
    const v = Math.max(lim[0], Math.min(lim[1], this.data[k] + (+d)))
    this.setData({ [k]: v }); this.calc()
  }
})
