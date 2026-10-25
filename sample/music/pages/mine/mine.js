const D = require('../../utils/songs.js')
const P = require('../../utils/player.js')
const S = require('../../utils/store.js')
Page({
  data: {
    tab: 0,
    favCount: 0,
    recentCount: 0,
    list: []
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 2 })
    this.refresh()
  },
  refresh() {
    const favs = S.favs()
    const recent = S.recent()
    const ids = this.data.tab === 0 ? favs : recent
    this.setData({
      favCount: favs.length,
      recentCount: recent.length,
      list: ids.map(id => D.song(id)).filter(Boolean).map(s => Object.assign({}, s, { fav: favs.indexOf(s.id) >= 0 }))
    })
  },
  onTab(e) {
    this.setData({ tab: Number(e.currentTarget.dataset.i) })
    this.refresh()
  },
  onSong(e) {
    P.playList(this.data.list.map(s => s.id), Number(e.currentTarget.dataset.index))
    wx.navigateTo({ url: '/pages/player/player' })
  },
  onUnfav(e) {
    S.toggleFav(this.data.list[Number(e.currentTarget.dataset.index)].id)
    this.refresh()
  },
  onClearRecent() {
    if (!this.data.recentCount) return
    wx.showModal({
      title: '清空最近播放',
      content: '确定清空全部 ' + this.data.recentCount + ' 首最近播放记录吗？',
      success: res => {
        if (!res.confirm) return
        S.clearRecent()
        this.refresh()
        wx.showToast({ title: '已清空', icon: 'none' })
      }
    })
  }
})
