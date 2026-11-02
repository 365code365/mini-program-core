// 寄件券：可用 / 已使用。下单时自动选最省的一张，也可以在寄件页手动换；取消寄件时券退回
const S = require('../../utils/store.js')
Page({
  data: { tab: 0, list: [], counts: [0, 0] },
  onShow() { this.load() },
  load() {
    const all = S.getCoupons()
    const valid = all.filter(c => c.status === 'valid')
    const used = all.filter(c => c.status !== 'valid')
    this.setData({
      counts: [valid.length, used.length],
      list: (this.data.tab === 0 ? valid : used).map(c => Object.assign({}, c, { cond: c.min ? '满 ' + c.min + ' 元可用' : '无门槛' }))
    })
  },
  onTab(e) {
    this.setData({ tab: Number(e.currentTarget.dataset.i) })
    this.load()
  },
  onUse() { wx.switchTab({ url: '/pages/send/send' }) }
})
