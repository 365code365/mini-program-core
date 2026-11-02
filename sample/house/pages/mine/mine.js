const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
Page({
  data: {
    profile: { name: '', need: '' },
    initial: '',
    favCount: 0,
    historyCount: 0,
    bookingCount: 0,
    pendingCount: 0,
    subCount: 0,
    bookings: [],
    menus: []
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 })
    const bookings = S.getBookings()
    const subs = S.getSubs()
    const entrusts = S.getEntrusts().filter(e => e.status !== 'withdrawn')
    this.setData({
      profile: S.getProfile(),
      initial: S.getProfile().name.slice(0, 1),
      favCount: S.getFavs().length,
      historyCount: S.getHistory().length,
      bookingCount: bookings.length,
      pendingCount: bookings.filter(b => b.status === 'pending').length,
      subCount: subs.length,
      // 卡片里只放最近两条，其余去「全部」
      bookings: bookings.slice(0, 2),
      menus: [
        { key: 'calc', text: '房贷计算器', glyph: '算', bg: '#ff6a1f', hint: 'LPR 3.5%' },
        { key: 'sell', text: '我要卖房', glyph: '卖', bg: '#3a86ff', hint: entrusts.length ? entrusts.length + ' 条委托' : '免费估价' },
        { key: 'sub', text: '房源订阅', glyph: '订', bg: '#19a974', hint: subs.length ? (subs[0].text + (subs.length > 1 ? ' 等' : '')) : '未订阅' },
        { key: 'price', text: '查房价', glyph: '价', bg: '#f5a300', hint: '各区均价' },
        { key: 'service', text: '联系客服', glyph: '服', bg: '#8b5cf6', hint: '400-810-6688' }
      ]
    })
  },
  onFavTab() {
    getApp().globalData.favTab = 0
    wx.switchTab({ url: '/pages/favorite/favorite' })
  },
  onHistoryTab() {
    getApp().globalData.favTab = 1
    wx.switchTab({ url: '/pages/favorite/favorite' })
  },
  onBookings() {
    wx.navigateTo({ url: '/pages/bookings/bookings' })
  },
  onSubs() {
    wx.navigateTo({ url: '/pages/subscribe/subscribe' })
  },
  onEdit() {
    const needs = S.NEEDS
    wx.showActionSheet({
      itemList: needs,
      success: res => {
        S.setNeed(needs[res.tapIndex])
        this.setData({ profile: S.getProfile() })
        wx.showToast({ title: '已更新购房需求', icon: 'none' })
      }
    })
  },
  onMenu(e) {
    const key = e.currentTarget.dataset.key
    if (key === 'calc') wx.navigateTo({ url: '/pages/calc/calc' })
    else if (key === 'sell') wx.navigateTo({ url: '/pages/sell/sell' })
    else if (key === 'sub') wx.navigateTo({ url: '/pages/subscribe/subscribe' })
    else if (key === 'price') wx.navigateTo({ url: '/pages/price/price' })
    else host.call('400 810 6688', '安居找房客服')
  }
})
