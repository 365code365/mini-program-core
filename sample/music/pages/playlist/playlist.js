const D = require('../../utils/songs.js')
const P = require('../../utils/player.js')
const S = require('../../utils/store.js')
const { withMini } = require('../../utils/mini.js')
Page(withMini({
  data: {
    miniOnTab: false,
    pl: null,
    songs: [],
    curId: 0,
    totalText: ''
  },
  onLoad(q) {
    const pl = D.playlist(q.id) || D.PLAYLISTS[0]
    const total = pl.ids.reduce((m, id) => m + D.song(id).duration, 0)
    this.ids = pl.ids
    this.setData({ pl, totalText: Math.round(total / 60) + ' 分钟' })
    wx.setNavigationBarTitle({ title: pl.name })
  },
  onShow() {
    const favs = S.favs()
    const cur = P.snapshot().song
    this.setData({
      songs: this.ids.map(id => Object.assign({}, D.song(id), { fav: favs.indexOf(id) >= 0 })),
      curId: cur ? cur.id : 0
    })
  },
  onPlayAll() {
    P.playList(this.ids, 0)
    wx.navigateTo({ url: '/pages/player/player' })
  },
  // 点歌：以本歌单为播放队列，从这首开始放，并进入播放页
  onSong(e) {
    const i = Number(e.currentTarget.dataset.index)
    P.playList(this.ids, i)
    this.setData({ curId: this.ids[i] })
    wx.navigateTo({ url: '/pages/player/player' })
  }
}))
