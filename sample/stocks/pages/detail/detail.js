const M = require('../../utils/market.js')
const A = require('../../utils/account.js')
Page({
  data: {
    code: '',
    q: null,
    tab: 0,
    tabs: ['分时', '日K'],
    pts: [],
    bars: [],
    book: { sells: [], buys: [] },
    watched: false,
    hold: 0
  },
  onLoad(query) {
    const code = M.info(query.code) ? query.code : '600519'
    this.setData({ code, bars: M.kline(code) })
    wx.setNavigationBarTitle({ title: M.info(code).name + ' ' + code })
  },
  onShow() {
    this.refresh()
  },
  onTick() {
    this.refresh()
  },
  refresh() {
    const code = this.data.code
    this.setData({
      q: M.quote(code),
      pts: this.data.tab === 0 ? M.intraday(code) : this.data.pts,
      book: M.book(code),
      watched: A.isWatched(code),
      hold: A.holding(code)
    })
  },
  onTab(e) {
    this.setData({ tab: Number(e.currentTarget.dataset.i) })
    this.refresh()
  },
  onWatch() {
    const code = this.data.code
    if (A.isWatched(code)) {
      A.removeWatch(code)
      wx.showToast({ title: '已删除自选', icon: 'none' })
    } else {
      A.addWatch(code)
      wx.showToast({ title: '已加入自选', icon: 'success' })
    }
    this.refresh()
  },
  onTrade(e) {
    wx.navigateTo({ url: '/pages/trade/trade?code=' + this.data.code + '&side=' + e.currentTarget.dataset.side })
  }
})
