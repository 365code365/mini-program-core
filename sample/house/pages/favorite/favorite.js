const D = require('../../utils/data.js')

Page({
  data: { tab: 0, favs: [], history: [], shown: [] },

  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 1 })
    this.refresh()
  },

  refresh() {
    const all = D.all()
    const pick = ids => ids.map(id => all.find(h => h.id === id)).filter(Boolean)
    const favs = pick(D.getFavs())
    const history = pick(wx.getStorageSync('house_history') || [])
    this.setData({ favs, history, shown: this.data.tab === 0 ? favs : history })
  },

  onTab(e) {
    this.setData({ tab: Number(e.currentTarget.dataset.t) })
    this.refresh()
  },

  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  },

  onRemove(e) {
    const id = e.currentTarget.dataset.id
    wx.showModal({
      title: '取消收藏',
      content: '确定不再关注这套房源了吗？',
      confirmColor: '#ff5a1f',
      success: res => {
        if (!res.confirm) return
        D.toggleFav(id)
        this.refresh()
      }
    })
  },

  onBook(e) {
    wx.navigateTo({ url: '/pages/booking/booking?id=' + e.currentTarget.dataset.id })
  },

  onGo() {
    wx.switchTab({ url: '/pages/index/index' })
  }
})
