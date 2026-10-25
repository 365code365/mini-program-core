const R = require('../../utils/recipes.js')
const S = require('../../utils/store.js')
Page({
  data: {
    favCount: 0,
    cookedTotal: 0,
    historyCount: 0,
    cooked: []
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 2 })
    this.refresh()
  },
  refresh() {
    const m = S.cooked()
    const cooked = Object.keys(m).map(id => {
      const r = R.byId(id)
      return r && { id: r.id, name: r.name, times: m[id], c1: r.c1, c2: r.c2, initial: r.initial }
    }).filter(Boolean).sort((a, b) => b.times - a.times)
    this.setData({
      favCount: S.favs().length,
      cookedTotal: cooked.reduce((n, x) => n + x.times, 0),
      historyCount: S.history().length,
      cooked
    })
  },
  onRecipe(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  },
  onClearHistory() {
    if (!this.data.historyCount) return wx.showToast({ title: '没有搜索历史', icon: 'none' })
    wx.showModal({
      title: '清空搜索历史',
      content: '共 ' + this.data.historyCount + ' 条，清空后不可恢复',
      success: res => {
        if (!res.confirm) return
        S.clearHistory()
        this.refresh()
      }
    })
  }
})
