const D = require('../../utils/data.js')

const SEED_BOOKING = {
  id: 1, houseId: 'h2', community: '西溪诚园', cover: '/images/h2_living.png', layout: '2室1厅1卫', area: 89,
  time: '10-19 周六 14:00-16:00', agent: '陆明轩', status: '已带看'
}

Page({
  data: {
    favCount: 0,
    historyCount: 0,
    bookings: [],
    menus: [
      { text: '房贷计算器', glyph: '算', bg: '#ff6a1f', hint: 'LPR 3.5%' },
      { text: '我要卖房', glyph: '卖', bg: '#3a86ff', hint: '免费估价' },
      { text: '房源订阅', glyph: '订', bg: '#19a974', hint: '滨江 · 三房' },
      { text: '联系客服', glyph: '服', bg: '#8b5cf6', hint: '400-810-6688' }
    ]
  },

  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 2 })
    this.setData({
      favCount: D.getFavs().length,
      historyCount: (wx.getStorageSync('house_history') || []).length,
      bookings: D.getBookings().concat([SEED_BOOKING])
    })
  },

  onFavTab() {
    wx.switchTab({ url: '/pages/favorite/favorite' })
  },

  onEdit() {
    wx.showActionSheet({
      itemList: ['刚需首套', '改善置换', '投资', '租房'],
      success: () => wx.showToast({ title: '已更新购房需求', icon: 'none' })
    })
  },

  onMenu(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (i === 0) {
      const r = 0.035 / 12, n = 360
      const pay = Math.round(3000000 * r * Math.pow(1 + r, n) / (Math.pow(1 + r, n) - 1))
      wx.showModal({ title: '房贷计算器', content: '贷款 300 万 · 30 年 · 等额本息\n月供约 ' + pay + ' 元', showCancel: false, confirmColor: '#ff5a1f' })
    } else if (i === 3) {
      wx.makePhoneCall({ phoneNumber: '4008106688', fail() {} })
    } else {
      wx.showToast({ title: '已提交，顾问稍后联系您', icon: 'none' })
    }
  }
})
