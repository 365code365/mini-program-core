const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
Page({
  data: { tab: 0, favs: [], history: [], shown: [] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 })
    // 「我的」里点「足迹」切过来时带的是浏览足迹（switchTab 不能带 query，走 globalData）
    const app = getApp()
    if (app.globalData.favTab !== undefined) {
      this.setData({ tab: app.globalData.favTab })
      delete app.globalData.favTab
    }
    this.refresh()
  },
  refresh() {
    const all = D.all()
    const pick = ids => ids.map(id => all.find(h => h.id === id)).filter(Boolean)
    const favs = pick(S.getFavs())
    const history = pick(S.getHistory()).map(h => Object.assign({}, h, { faved: S.isFav(h.id) }))
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
        if (S.isFav(id)) S.toggleFav(id)
        wx.showToast({ title: '已取消收藏', icon: 'none' })
        this.refresh()
      }
    })
  },
  // 长按：收藏页里取消收藏 / 约看；足迹里删除这条 / 收藏或取消收藏
  onLong(e) {
    const id = e.currentTarget.dataset.id
    const faved = S.isFav(id)
    const items = this.data.tab === 0
      ? ['取消收藏', '预约看房']
      : ['删除这条足迹', faved ? '取消收藏' : '加入收藏', '预约看房']
    wx.showActionSheet({
      itemList: items,
      success: res => {
        const act = items[res.tapIndex]
        if (act === '取消收藏') {
          if (S.isFav(id)) S.toggleFav(id)
          wx.showToast({ title: '已取消收藏', icon: 'none' })
        } else if (act === '加入收藏') {
          S.toggleFav(id)
          wx.showToast({ title: '已加入收藏', icon: 'none' })
        } else if (act === '删除这条足迹') {
          S.removeHistory(id)
        } else {
          return this.book(id)
        }
        this.refresh()
      }
    })
  },
  onClear() {
    wx.showModal({
      title: '清空浏览足迹',
      content: '清空后不可恢复，收藏不受影响。',
      confirmText: '清空',
      confirmColor: '#ff5a1f',
      success: r => {
        if (!r.confirm) return
        S.clearHistory()
        this.refresh()
      }
    })
  },
  book(id) {
    if (S.pendingFor(id)) return wx.navigateTo({ url: '/pages/bookings/bookings?tab=pending' })
    wx.navigateTo({ url: '/pages/booking/booking?id=' + id })
  },
  onBook(e) {
    this.book(e.currentTarget.dataset.id)
  },
  onGo() {
    wx.switchTab({ url: '/pages/index/index' })
  }
})
