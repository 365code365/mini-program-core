const R = require('../../utils/recipes.js')
const S = require('../../utils/store.js')
Page({
  data: {
    list: []
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 1 })
    this.refresh()
  },
  refresh() {
    const list = S.sortedFavs().map(f => {
      const r = R.byId(f.id)
      return r && { id: r.id, name: r.name, minutes: r.minutes, level: r.level, catText: r.catText, c1: r.c1, c2: r.c2, initial: r.initial, pinned: !!f.pinned }
    }).filter(Boolean)
    this.setData({ list })
  },
  onRecipe(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  },
  // 长按：操作菜单（置顶 / 取消置顶、删除）
  onLong(e) {
    const item = this.data.list[Number(e.currentTarget.dataset.index)]
    wx.showActionSheet({
      itemList: [item.pinned ? '取消置顶' : '置顶', '删除'],
      success: res => {
        if (res.tapIndex === 0) {
          const on = S.togglePin(item.id)
          wx.showToast({ title: on ? '已置顶' : '已取消置顶', icon: 'none' })
        } else if (res.tapIndex === 1) {
          S.removeFav(item.id)
          wx.showToast({ title: '已删除「' + item.name + '」', icon: 'none' })
        }
        this.refresh()
      }
    })
  },
  onGo() {
    wx.switchTab({ url: '/pages/index/index' })
  }
})
