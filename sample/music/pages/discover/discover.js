const D = require('../../utils/songs.js')
const P = require('../../utils/player.js')
const { withMini } = require('../../utils/mini.js')
// 每日推荐：按日期轮换起点，同一天固定
const DAILY = [1, 5, 8, 11, 6, 13]
Page(withMini({
  data: {
    miniOnTab: true,
    greet: '',
    playlists: D.PLAYLISTS,
    daily: DAILY.map(id => D.song(id)),
    curId: 0
  },
  onLoad() {
    const h = new Date().getHours()
    this.setData({ greet: h < 11 ? '早上好' : h < 18 ? '下午好' : '晚上好' })
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 0 })
    const s = P.snapshot().song
    this.setData({ curId: s ? s.id : 0 })
  },
  onPlaylist(e) {
    wx.navigateTo({ url: '/pages/playlist/playlist?id=' + e.currentTarget.dataset.id })
  },
  onDaily(e) {
    const i = Number(e.currentTarget.dataset.index)
    P.playList(DAILY, i)
    this.setData({ curId: DAILY[i] })
  },
  onPlayDaily() {
    P.playList(DAILY, 0)
    wx.navigateTo({ url: '/pages/player/player' })
  }
}))
